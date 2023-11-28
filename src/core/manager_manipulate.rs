use anyhow::Result;
use nihility_common::manipulate::ManipulateType;
use nihility_common::response_code::RespCode;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info};

use crate::entity::manipulate::ManipulateEntity;

use super::SUBMODULE_MAP;

/// 处理操作的接收和转发
///
/// 1、通过操作实体转发操作
///
/// 2、记录日志
///
/// 3、处理特定的错误
pub(super) async fn manager_manipulate(
    mut manipulate_receiver: UnboundedReceiver<ManipulateEntity>,
) -> Result<()> {
    info!("Start Receive Manipulate");
    while let Some(manipulate) = manipulate_receiver.recv().await {
        info!("Get Manipulate：{:?}", &manipulate);
        match manipulate.manipulate_type {
            ManipulateType::OfflineType => {
                error!("Offline Type Manipulate Cannot Forward")
            }
            _ => {}
        }
        let mut locked_module_map = SUBMODULE_MAP.lock().await;
        if let Some(module) = locked_module_map.get_mut(manipulate.use_module_name.as_str()) {
            match module.send_manipulate(manipulate).await {
                Ok(RespCode::Success) => {
                    debug!("Send Manipulate Success");
                }
                Ok(other_resp_code) => {
                    error!("Send Manipulate Fail, Resp Code: {:?}", other_resp_code);
                }
                Err(e) => {
                    error!("Send Manipulate Error: {}", e);
                }
            }
        } else {
            error!(
                "Expect Use Submodule Name {:?} Cannot Find In Register Submodule",
                manipulate.use_module_name.to_string()
            )
        }
    }
    Ok(())
}
