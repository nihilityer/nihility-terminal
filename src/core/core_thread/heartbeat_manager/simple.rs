use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use nihility_common::{ModuleOperate, OperateType};
use tokio::sync::mpsc::WeakUnboundedSender;
use tokio::{select, spawn};
use tracing::{debug, error, info};

use crate::core::core_thread::heartbeat_manager::HEARTBEAT_TIME;
use crate::core::submodule_store::SubmoduleStore;
use crate::{CANCELLATION_TOKEN, CLOSE_SENDER};

pub fn simple_heartbeat_manager_thread(
    submodule_store: Arc<Box<dyn SubmoduleStore + Send + Sync>>,
    sender: WeakUnboundedSender<ModuleOperate>,
) -> Result<()> {
    let close_sender = CLOSE_SENDER.get().unwrap().upgrade().unwrap();
    spawn(async move {
        select! {
            heartbeat_result = start(submodule_store, sender) => {
                if let Err(e) = heartbeat_result {
                    error!("Heartbeat Manager Thread Error: {}", e);
                    CANCELLATION_TOKEN.cancel();
                }
            },
            _ = CANCELLATION_TOKEN.cancelled() => {}
        }
        close_sender
            .send("Manager Heartbeat Thread".to_string())
            .await
            .unwrap();
    });
    Ok(())
}

/// 管理子模块的心跳，当有子模块心跳过期时
///
/// 通过`module_operate_sender`发送消息将对于子模块离线
async fn start(
    submodule_store: Arc<Box<dyn SubmoduleStore + Send + Sync>>,
    sender: WeakUnboundedSender<ModuleOperate>,
) -> Result<()> {
    loop {
        tokio::time::sleep(Duration::from_secs(HEARTBEAT_TIME)).await;
        debug!("Make Sure The Submodule Heartbeat Is Normal");
        for name in submodule_store
            .get_expire_heartbeat_submodule(HEARTBEAT_TIME)
            .await?
        {
            info!("Submodule {:?} Heartbeat Exception", &name);
            let operate = ModuleOperate {
                name,
                info: None,
                operate_type: OperateType::Offline,
            };
            sender.upgrade().unwrap().send(operate)?;
        }
    }
}
