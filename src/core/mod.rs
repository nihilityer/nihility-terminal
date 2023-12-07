use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, WeakUnboundedSender};
use tracing::info;

use crate::config::CoreConfig;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::SimpleManipulateEntity;
use crate::entity::submodule::{ModuleOperate, Submodule};

pub mod encoder;
pub mod instruct_manager;
mod manager_heartbeat;
mod manager_instruct;
mod manager_manipulate;
mod manager_submodule;

const HEARTBEAT_TIME: u64 = 30;

lazy_static! {
    static ref SUBMODULE_MAP: tokio::sync::Mutex<HashMap<String, Submodule>> =
        tokio::sync::Mutex::new(HashMap::<String, Submodule>::new());
    static ref INSTRUCT_ENCODER: Mutex<Box<dyn encoder::InstructEncoder + Send + Sync>> =
        Mutex::new(Box::<encoder::mock::MockInstructEncoder>::default());
    static ref INSTRUCT_MANAGER: tokio::sync::Mutex<Box<dyn instruct_manager::InstructManager + Send + Sync>> =
        tokio::sync::Mutex::new(Box::<instruct_manager::mock::MockInstructManager>::default());
}

pub async fn core_start(
    mut core_config: CoreConfig,
    shutdown_sender: UnboundedSender<String>,
    module_operate_sender: WeakUnboundedSender<ModuleOperate>,
    module_operate_receiver: UnboundedReceiver<ModuleOperate>,
    instruct_receiver: UnboundedReceiver<InstructEntity>,
    manipulate_receiver: UnboundedReceiver<SimpleManipulateEntity>,
) -> Result<()> {
    info!("Core Start Init");

    if let Ok(mut locked_encoder) = INSTRUCT_ENCODER.lock() {
        *locked_encoder = encoder::encoder_builder(&core_config.encoder)?;
        let encode_size = locked_encoder.encode_size();
        core_config.module_manager.config_map.insert(
            instruct_manager::ENCODE_SIZE_FIELD.to_string(),
            encode_size.to_string(),
        );
    } else {
        return Err(anyhow!("Lock Instruct Encoder Error"));
    }

    {
        let mut locked_instruct_manager = INSTRUCT_MANAGER.lock().await;
        *locked_instruct_manager =
            instruct_manager::build_instruct_manager(core_config.module_manager).await?;
    }

    manager_submodule::start(shutdown_sender.clone(), module_operate_receiver);

    manager_heartbeat::start(shutdown_sender.clone(), module_operate_sender);

    manager_instruct::start(shutdown_sender.clone(), instruct_receiver);

    manager_manipulate::start(shutdown_sender.clone(), manipulate_receiver);

    Ok(())
}
