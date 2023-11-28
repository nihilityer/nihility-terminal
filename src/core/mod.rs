use std::collections::HashMap;

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
    let encoder = encoder::encoder_builder(&core_config.encoder)?;

    if let Ok(locked_encoder) = encoder.lock() {
        let encode_size = locked_encoder.encode_size();
        core_config.module_manager.config_map.insert(
            instruct_manager::ENCODE_SIZE_FIELD.to_string(),
            encode_size.to_string(),
        );
    } else {
        return Err(anyhow!("Lock Instruct Encoder Error"));
    }

    let built_instruct_manager =
        instruct_manager::build_instruct_manager(core_config.module_manager).await?;

    let submodule_built_instruct_manager = built_instruct_manager.clone();
    let submodule_encoder = encoder.clone();
    let submodule_cancellation_token = cancellation_token.clone();
    let submodule_shutdown_sender = shutdown_sender.clone();
    spawn(async move {
        if let Err(e) = manager_submodule::manager_submodule(
            submodule_built_instruct_manager,
            submodule_encoder,
            module_operate_receiver,
        )
        .await
        {
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

    let instruct_built_instruct_manager = built_instruct_manager.clone();
    let instruct_encoder = encoder.clone();
    let instruct_cancellation_token = cancellation_token.clone();
    let instruct_shutdown_sender = shutdown_sender.clone();
    spawn(async move {
        if let Err(e) = manager_instruct::manager_instruct(
            instruct_built_instruct_manager,
            instruct_encoder,
            instruct_receiver,
        )
        .await
        {
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
