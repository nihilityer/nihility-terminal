use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use nihility_common::{ModuleOperate, OperateType};
use tokio::sync::mpsc::{channel, Receiver, Sender, WeakUnboundedSender};
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
    let (se, re) = channel(1);
    spawn(async move {
        select! {
            heartbeat_result = start(submodule_store, re, sender) => {
                if let Err(e) = heartbeat_result {
                    error!("Heartbeat Manager Thread Error: {}", e);
                    CANCELLATION_TOKEN.cancel();
                }
            },
            _ = CANCELLATION_TOKEN.cancelled() => {},
            _ = send(se) => {}
        }
        close_sender
            .send("Heartbeat Manager Thread".to_string())
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
    mut heartbeat_receiver: Receiver<bool>,
    sender: WeakUnboundedSender<ModuleOperate>,
) -> Result<()> {
    info!("Heartbeat Manager Thread Start");
    while (heartbeat_receiver.recv().await).is_some() {
        debug!("Make Sure The Submodule Heartbeat Is Normal");
        for name in submodule_store
            .get_expire_heartbeat_submodule(HEARTBEAT_TIME * 2)
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
    Ok(())
}

async fn send(sender: Sender<bool>) {
    loop {
        tokio::time::sleep(Duration::from_secs(HEARTBEAT_TIME)).await;
        sender.send(true).await.unwrap();
    }
}
