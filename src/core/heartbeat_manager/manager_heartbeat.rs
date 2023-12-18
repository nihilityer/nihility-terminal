use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use tokio::sync::mpsc::{UnboundedSender, WeakUnboundedSender};
use tokio::{select, spawn};
use tracing::{debug, error, info};

use crate::entity::submodule::{ModuleOperate, OperateType};
use crate::CANCELLATION_TOKEN;

use super::{HEARTBEAT_TIME, SUBMODULE_MAP};

pub(super) fn start(
    shutdown_sender: UnboundedSender<String>,
    module_operate_sender: WeakUnboundedSender<ModuleOperate>,
) {
    spawn(async move {
        select! {
            heartbeat_result = manager_heartbeat(module_operate_sender) => {
                if let Err(e) = heartbeat_result {
                    error!("Heartbeat Manager Error: {}", e);
                    CANCELLATION_TOKEN.cancel();
                }
            },
            _ = CANCELLATION_TOKEN.cancelled() => {}
        }
        shutdown_sender
            .send("Heartbeat Manager".to_string())
            .unwrap();
    });
}

/// 管理子模块的心跳，当有子模块心跳过期时
///
/// 通过`module_operate_sender`发送消息将对于子模块离线
async fn manager_heartbeat(
    module_operate_sender: WeakUnboundedSender<ModuleOperate>,
) -> Result<()> {
    loop {
        tokio::time::sleep(Duration::from_secs(HEARTBEAT_TIME)).await;
        debug!("Make Sure The Submodule Heartbeat Is Normal");
        let mut locked_submodule_map = SUBMODULE_MAP.lock().await;
        let now_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        for (_, submodule) in locked_submodule_map.iter_mut() {
            if now_timestamp - submodule.heartbeat_time > 2 * HEARTBEAT_TIME {
                info!("Submodule {:?} Heartbeat Exception", &submodule.name);
                module_operate_sender.upgrade().unwrap().send(
                    ModuleOperate::create_by_submodule(submodule, OperateType::Offline),
                )?;
            }
        }
    }
}
