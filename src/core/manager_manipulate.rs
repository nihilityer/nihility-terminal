use anyhow::Result;
use nihility_common::manipulate::ManipulateType;
use nihility_common::response_code::RespCode;
use tokio::spawn;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::{debug, error, info};

use crate::entity::manipulate::SimpleManipulateEntity;
use crate::CANCELLATION_TOKEN;

use super::SUBMODULE_MAP;

pub(super) fn start(
    shutdown_sender: UnboundedSender<String>,
    manipulate_receiver: UnboundedReceiver<SimpleManipulateEntity>,
) {
    spawn(async move {
        if let Err(e) = manager_manipulate(manipulate_receiver).await {
            error!("Manipulate Manager Error: {}", e);
            CANCELLATION_TOKEN.cancel();
        }
        shutdown_sender
            .send("Manipulate Manager".to_string())
            .unwrap();
    });
}

/// 处理操作的接收和转发
///
/// 1、通过操作实体转发操作
///
/// 2、记录日志
///
/// 3、处理特定的错误
async fn manager_manipulate(
    mut manipulate_receiver: UnboundedReceiver<SimpleManipulateEntity>,
) -> Result<()> {
    info!("Start Receive Manipulate");
    while let Some(manipulate) = manipulate_receiver.recv().await {
        info!("Get Manipulate：{:?}", &manipulate);
        let manipulate_info = match manipulate.info {
            None => continue,
            Some(info) => info,
        };
        if manipulate_info.manipulate_type == ManipulateType::OfflineType {
            error!("Offline Type Manipulate Cannot Forward")
        }
        let mut locked_module_map = SUBMODULE_MAP.lock().await;
        if let Some(module) = locked_module_map.get_mut(manipulate_info.use_module_name.as_str()) {
            match module
                .send_manipulate(manipulate_info.create_simple_manipulate_entity())
                .await
            {
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
                manipulate_info.use_module_name.to_string()
            )
        }
    }
    Ok(())
}
