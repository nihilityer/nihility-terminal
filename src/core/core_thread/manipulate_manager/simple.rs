use anyhow::Result;
use nihility_common::{ManipulateData, ManipulateEntity, ManipulateType, ResponseCode};
use tokio::spawn;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info};

use crate::core::{OperationRecorderImpl, SubmoduleStoreImpl};
use crate::{CANCELLATION_TOKEN, CLOSE_SENDER};

pub fn simple_manipulate_manager_thread(
    submodule_store: SubmoduleStoreImpl,
    operation_recorder: OperationRecorderImpl,
    manipulate_receiver: UnboundedReceiver<ManipulateEntity>,
) -> Result<()> {
    let close_sender = CLOSE_SENDER.get().unwrap().upgrade().unwrap();
    spawn(async move {
        if let Err(e) = start(submodule_store, operation_recorder, manipulate_receiver).await {
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

async fn start(
    submodule_store: SubmoduleStoreImpl,
    operation_recorder: OperationRecorderImpl,
    mut manipulate_receiver: UnboundedReceiver<ManipulateEntity>,
) -> Result<()> {
    info!("Manipulate Manager Thread Start");
    while let Some(manipulate) = manipulate_receiver.recv().await {
        info!("Get Manipulate：{:?}", &manipulate);
        operation_recorder.recorder_manipulate(&manipulate).await?;
        if let ManipulateType::OfflineType = &manipulate.info.manipulate_type {
            error!("Offline Type Manipulate Cannot Forward")
        }
        if let Some(module) = submodule_store
            .lock()
            .await
            .get(&manipulate.info.use_module_name)
            .await?
        {
            match &manipulate.manipulate {
                ManipulateData::Text(_) => {
                    match module.client.text_display_manipulate(manipulate).await {
                        Ok(resp) => match resp.code() {
                            ResponseCode::Success => debug!("Send Text Display Manipulate Success"),
                            other_resp_code => error!(
                                "Send Text Display Manipulate Fail, Resp Code: {:?}",
                                other_resp_code
                            ),
                        },
                        Err(e) => error!("Send Text Display Manipulate Error: {}", e),
                    }
                }
                ManipulateData::Simple => match module.client.simple_manipulate(manipulate).await {
                    Ok(resp) => match resp.code() {
                        ResponseCode::Success => debug!("Send Simple Manipulate Success"),
                        other_resp_code => error!(
                            "Send Simple Manipulate Fail, Resp Code: {:?}",
                            other_resp_code
                        ),
                    },
                    Err(e) => error!("Send Simple Manipulate Error: {}", e),
                },
                ManipulateData::ConnectionParams(_) => {
                    match module.client.direct_connection_manipulate(manipulate).await {
                        Ok(resp) => match resp.code() {
                            ResponseCode::Success => {
                                debug!("Send Direct Connection Manipulate Success")
                            }
                            other_resp_code => error!(
                                "Send Direct Connection Manipulate Fail, Resp Code: {:?}",
                                other_resp_code
                            ),
                        },
                        Err(e) => error!("Send Direct Connection Manipulate Error: {}", e),
                    }
                }
            }
        } else {
            error!(
                "Expect Use Submodule Name {:?} Cannot Find In Register Submodule",
                &manipulate.info.use_module_name
            )
        }
    }
    Ok(())
}
