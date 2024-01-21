extern crate nihility_common;

use std::sync::OnceLock;

use anyhow::Result;
use lazy_static::lazy_static;
use nihility_common::{
    core_authentication_core_init, InstructEntity, ManipulateEntity, ModuleOperate,
};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{WeakSender, WeakUnboundedSender};
use tokio_util::sync::CancellationToken;

use crate::check::check;
pub use crate::config::NihilityTerminalConfig;
use crate::config::{
    HeartbeatManagerType, InstructEncoderType, InstructManagerType, InstructMatcherType,
    ManipulateManagerType, SubmoduleManagerType, SubmoduleStoreType,
};
use crate::core::core_thread::heartbeat_manager::simple_heartbeat_manager_thread;
use crate::core::core_thread::instruct_manager::simple_instruct_manager_thread;
use crate::core::core_thread::manipulate_manager::simple_manipulate_manager_thread;
use crate::core::core_thread::submodule_manager::simple_submodule_manager_thread;
use crate::core::instruct_encoder::sentence_transformers::SentenceTransformers;
use crate::core::instruct_encoder::InstructEncoder;
use crate::core::instruct_matcher::grpc_qdrant::GrpcQdrant;
use crate::core::instruct_matcher::InstructMatcher;
use crate::core::submodule_store::{HashMapSubmoduleStore, SubmoduleStore};
use crate::core::{NihilityCore, NihilityCoreBuilder};

pub mod check;
mod config;
mod core;
mod entity;
mod server;

lazy_static! {
    static ref CANCELLATION_TOKEN: CancellationToken = CancellationToken::new();
}
static CLOSE_SENDER: OnceLock<WeakSender<String>> = OnceLock::new();
static MODULE_OPERATE_SENDER: OnceLock<WeakUnboundedSender<ModuleOperate>> = OnceLock::new();
static INSTRUCT_SENDER: OnceLock<WeakUnboundedSender<InstructEntity>> = OnceLock::new();
static MANIPULATE_SENDER: OnceLock<WeakUnboundedSender<ManipulateEntity>> = OnceLock::new();

pub struct NihilityTerminal;

impl NihilityTerminal {
    pub fn set_close_sender(close_sender: WeakSender<String>) {
        CLOSE_SENDER.get_or_init(|| close_sender);
    }

    pub fn get_cancellation_token() -> CancellationToken {
        CANCELLATION_TOKEN.clone()
    }

    pub async fn start(summary_config: NihilityTerminalConfig) -> Result<()> {
        core_authentication_core_init(&summary_config.core.auth_key_dir)?;

        check()?;

        let (module_operate_se, module_operate_re) = mpsc::unbounded_channel::<ModuleOperate>();
        let (instruct_se, instruct_re) = mpsc::unbounded_channel::<InstructEntity>();
        let (manipulate_se, manipulate_re) = mpsc::unbounded_channel::<ManipulateEntity>();

        MODULE_OPERATE_SENDER.get_or_init(|| module_operate_se.downgrade());
        INSTRUCT_SENDER.get_or_init(|| instruct_se.downgrade());
        MANIPULATE_SENDER.get_or_init(|| manipulate_se.downgrade());

        server::server_start(&summary_config.server).await?;

        let mut core_builder = NihilityCoreBuilder::default();

        core_builder.set_instruct_receiver(instruct_re);
        core_builder.set_manipulate_receiver(manipulate_re);
        core_builder.set_module_operate_receiver(module_operate_re);

        match &summary_config.core.instruct_encoder.instruct_encoder_type {
            InstructEncoderType::SentenceTransformers => {
                core_builder.set_instruct_encoder(Box::new(SentenceTransformers::init(
                    &summary_config.core.instruct_encoder,
                )?));
            }
        }

        match &summary_config.core.instruct_matcher.instruct_matcher_type {
            InstructMatcherType::GrpcQdrant => {
                core_builder.set_instruct_matcher(Box::new(
                    GrpcQdrant::init(&summary_config.core.instruct_matcher).await?,
                ));
            }
        }

        match &summary_config.core.submodule_store.submodule_store_type {
            SubmoduleStoreType::SimpleHashMap => {
                core_builder.set_submodule_store(Box::new(
                    HashMapSubmoduleStore::init(&summary_config.core.submodule_store).await?,
                ));
            }
        }

        match &summary_config.core.heartbeat_manager {
            HeartbeatManagerType::Simple => {
                core_builder.set_heartbeat_manager_fn(simple_heartbeat_manager_thread);
            }
        }

        match &summary_config.core.instruct_manager {
            InstructManagerType::Simple => {
                core_builder.set_instruct_manager_fn(simple_instruct_manager_thread);
            }
        }

        match &summary_config.core.manipulate_manager {
            ManipulateManagerType::Simple => {
                core_builder.set_manipulate_manager_fn(simple_manipulate_manager_thread);
            }
        }

        match &summary_config.core.submodule_manager {
            SubmoduleManagerType::Simple => {
                core_builder.set_submodule_manager_fn(simple_submodule_manager_thread);
            }
        }

        NihilityCore::build(core_builder)?;

        drop(instruct_se);
        drop(manipulate_se);
        drop(module_operate_se);

        Ok(())
    }
}
