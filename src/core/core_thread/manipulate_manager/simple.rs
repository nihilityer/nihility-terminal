use std::sync::Arc;

use anyhow::Result;
use nihility_common::{ManipulateEntity, ManipulateType, ResponseCode};
use tokio::spawn;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info};

use crate::core::submodule_store::SubmoduleStore;
use crate::{CANCELLATION_TOKEN, CLOSE_SENDER};

pub fn simple_manipulate_manager_thread(
    submodule_store: Arc<Box<dyn SubmoduleStore + Send + Sync>>,
    manipulate_receiver: UnboundedReceiver<ManipulateEntity>,
) -> Result<()> {
    let close_sender = CLOSE_SENDER.get().unwrap().upgrade().unwrap();
    spawn(async move {
        if let Err(e) = start(submodule_store, manipulate_receiver).await {
            error!("Manipulate Manager Thread Error: {}", e);
            CANCELLATION_TOKEN.cancel();
        }
        close_sender
            .send("Manipulate Manager Thread".to_string())
            .await
            .unwrap();
    });
    Ok(())
}

/// 处理操作的接收和转发
///
/// 1、通过操作实体转发操作
///
/// 2、记录日志
///
/// 3、处理特定的错误
async fn start(
    submodule_store: Arc<Box<dyn SubmoduleStore + Send + Sync>>,
    mut manipulate_receiver: UnboundedReceiver<ManipulateEntity>,
) -> Result<()> {
    info!("Manipulate Manager Thread Start");
    while let Some(manipulate) = manipulate_receiver.recv().await {
        info!("Get Manipulate：{:?}", &manipulate);
        if let ManipulateType::OfflineType = &manipulate.info.manipulate_type {
            error!("Offline Type Manipulate Cannot Forward")
        }
        if let Some(module) = submodule_store
            .get_and_remove(&manipulate.info.use_module_name)
            .await?
        {
            match module.client.text_display_manipulate(manipulate).await {
                Ok(ResponseCode::Success) => {
                    debug!("Send Manipulate Success");
                }
                Ok(other_resp_code) => {
                    error!("Send Manipulate Fail, Resp Code: {:?}", other_resp_code);
                }
                Err(e) => {
                    error!("Send Manipulate Error: {}", e);
                }
            }
            submodule_store.insert(module).await?;
        } else {
            error!(
                "Expect Use Submodule Name {:?} Cannot Find In Register Submodule",
                &manipulate.info.use_module_name
            )
        }
    }
    Ok(())
}
