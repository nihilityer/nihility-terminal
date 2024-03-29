use anyhow::Result;
use nihility_common::InstructData::Text;
use nihility_common::{InstructEntity, ResponseCode};
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info, warn};

use crate::core::{
    InstructEncoderImpl, InstructMatcherImpl, OperationRecorderImpl, SubmoduleStoreImpl,
};

pub async fn simple_instruct_manager_thread(
    instruct_encoder: InstructEncoderImpl,
    instruct_matcher: InstructMatcherImpl,
    submodule_store: SubmoduleStoreImpl,
    operation_recorder: OperationRecorderImpl,
    mut instruct_receiver: UnboundedReceiver<InstructEntity>,
) -> Result<()> {
    info!("Instruct Manager Thread Start");
    while let Some(instruct) = instruct_receiver.recv().await {
        info!("Get Instruct：{:?}", &instruct);
        operation_recorder.recorder_instruct(&instruct).await?;
        let mut encoded_instruct: Vec<f32> = Vec::new();
        match &instruct.instruct {
            Text(text) => {
                encoded_instruct.append(instruct_encoder.encode(text).await?.as_mut());
            }
        }

        match instruct_matcher.lock().await.search(encoded_instruct).await {
            Ok(module_name) => {
                if let Some(module) = submodule_store.lock().await.get(&module_name).await? {
                    match module.client.text_instruct(instruct).await {
                        Ok(resp) => match resp.code() {
                            ResponseCode::Success => debug!("Forward Instruct Success"),
                            other_resp_code => {
                                error!("Forward Instruct Fail, Resp Code: {:?}", other_resp_code)
                            }
                        },
                        Err(e) => error!("Forward Instruct Error: {}", e),
                    }
                    continue;
                }
            }
            Err(e) => {
                warn!("Match Instruct Handler Error: {}", e);
            }
        }
    }
    Ok(())
}
