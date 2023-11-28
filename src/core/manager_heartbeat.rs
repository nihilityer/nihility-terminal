use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use tokio::sync::mpsc::WeakUnboundedSender;
use tracing::{debug, info};

use crate::entity::module::{ModuleOperate, OperateType};

use super::{HEARTBEAT_TIME, SUBMODULE_MAP};

/// 管理子模块的心跳，当有子模块心跳过期时
///
/// 通过`module_operate_sender`发送消息将对于子模块离线
pub(super) async fn manager_heartbeat(
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
                    ModuleOperate::create_by_submodule(&submodule, OperateType::OFFLINE),
                )?;
            }
        }
    }
}
