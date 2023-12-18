use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use anyhow::Result;
use lazy_static::lazy_static;
use nihility_common::manipulate::ManipulateType;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, WeakUnboundedSender};
use tracing::info;

use crate::config::CoreConfig;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::{ManipulateEntity, ManipulateInfoEntity};
use crate::entity::submodule::{ModuleOperate, Submodule};

mod manager_instruct;
mod manager_manipulate;
mod manager_submodule;
pub mod instruct_encoder;
pub mod instruct_matcher;
mod heartbeat_manager;

const HEARTBEAT_TIME: u64 = 30;

lazy_static! {
    static ref SUBMODULE_MAP: tokio::sync::Mutex<HashMap<String, Submodule>> =
        tokio::sync::Mutex::new(HashMap::<String, Submodule>::new());
}

static INSTRUCT_MANAGER: OnceLock<
    tokio::sync::Mutex<Box<dyn instruct_manager::InstructManager + Send + Sync>>,
> = OnceLock::new();
static INSTRUCT_ENCODER: OnceLock<Mutex<Box<dyn encoder::InstructEncoder + Send + Sync>>> =
    OnceLock::new();
static DEFAULT_MANIPULATE_INFO: OnceLock<ManipulateInfoEntity> = OnceLock::new();

pub async fn core_start(
    mut core_config: CoreConfig,
    shutdown_sender: UnboundedSender<String>,
    module_operate_sender: WeakUnboundedSender<ModuleOperate>,
    module_operate_receiver: UnboundedReceiver<ModuleOperate>,
    instruct_receiver: UnboundedReceiver<InstructEntity>,
    manipulate_receiver: UnboundedReceiver<ManipulateEntity>,
) -> Result<()> {
    info!("Core Start Init");

    DEFAULT_MANIPULATE_INFO.get_or_init(|| ManipulateInfoEntity {
        manipulate_type: ManipulateType::DefaultType,
        use_module_name: core_config.default_use_submodule.clone(),
    });

    let encoder = encoder::encoder_builder(&core_config.encoder)?;
    core_config.module_manager.config_map.insert(
        instruct_manager::ENCODE_SIZE_FIELD.to_string(),
        encoder.encode_size().to_string(),
    );
    INSTRUCT_ENCODER.get_or_init(|| {
        Mutex::new(encoder)
    });

    let instruct_manager =
        instruct_manager::build_instruct_manager(core_config.module_manager).await?;
    INSTRUCT_MANAGER.get_or_init(|| tokio::sync::Mutex::new(instruct_manager));

    manager_submodule::start(shutdown_sender.clone(), module_operate_receiver);

    manager_heartbeat::start(shutdown_sender.clone(), module_operate_sender);

    manager_instruct::start(shutdown_sender.clone(), instruct_receiver);

    manager_manipulate::start(shutdown_sender.clone(), manipulate_receiver);

    Ok(())
}
