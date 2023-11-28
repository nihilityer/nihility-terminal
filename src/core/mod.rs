use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, WeakUnboundedSender};
use tokio::{select, spawn};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::config::CoreConfig;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::{ModuleOperate, Submodule};

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
        Mutex::new(Box::new(encoder::mock::MockInstructEncoder::default()));
    static ref INSTRUCT_MANAGER: tokio::sync::Mutex<Box<dyn instruct_manager::InstructManager + Send + Sync>> =
        tokio::sync::Mutex::new(Box::new(
            instruct_manager::mock::MockInstructManager::default()
        ));
}

pub async fn core_start(
    mut core_config: CoreConfig,
    cancellation_token: CancellationToken,
    shutdown_sender: UnboundedSender<String>,
    module_operate_sender: WeakUnboundedSender<ModuleOperate>,
    module_operate_receiver: UnboundedReceiver<ModuleOperate>,
    instruct_receiver: UnboundedReceiver<InstructEntity>,
    manipulate_receiver: UnboundedReceiver<ManipulateEntity>,
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

    let submodule_cancellation_token = cancellation_token.clone();
    let submodule_shutdown_sender = shutdown_sender.clone();
    spawn(async move {
        if let Err(e) = manager_submodule::manager_submodule(module_operate_receiver).await {
            error!("Submodule Manager Error: {}", e);
            submodule_cancellation_token.cancel();
        }
        submodule_shutdown_sender
            .send("Submodule Manager".to_string())
            .unwrap();
    });

    let heartbeat_cancellation_token = cancellation_token.clone();
    let heartbeat_shutdown_sender = shutdown_sender.clone();
    spawn(async move {
        select! {
            heartbeat_result = manager_heartbeat::manager_heartbeat(module_operate_sender) => {
                match heartbeat_result {
                    Err(e) => {
                        error!("Heartbeat Manager Error: {}", e);
                        heartbeat_cancellation_token.cancel();
                    }
                    _ => {}
                }
            },
            _ = heartbeat_cancellation_token.cancelled() => {}
        }
        heartbeat_shutdown_sender
            .send("Heartbeat Manager".to_string())
            .unwrap();
    });

    let instruct_cancellation_token = cancellation_token.clone();
    let instruct_shutdown_sender = shutdown_sender.clone();
    spawn(async move {
        if let Err(e) = manager_instruct::manager_instruct(instruct_receiver).await {
            error!("Instruct Manager Error: {}", e);
            instruct_cancellation_token.cancel();
        }
        instruct_shutdown_sender
            .send("Instruct Manager".to_string())
            .unwrap();
    });

    let manipulate_cancellation_token = cancellation_token.clone();
    let manipulate_shutdown_sender = shutdown_sender.clone();
    spawn(async move {
        if let Err(e) = manager_manipulate::manager_manipulate(manipulate_receiver).await {
            error!("Manipulate Manager Error: {}", e);
            manipulate_cancellation_token.cancel();
        }
        manipulate_shutdown_sender
            .send("Manipulate Manager".to_string())
            .unwrap();
    });

    Ok(())
}
