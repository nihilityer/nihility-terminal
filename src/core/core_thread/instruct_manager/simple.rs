use anyhow::Result;
use nihility_common::InstructData::Text;
use nihility_common::{InstructEntity, ResponseCode};
use tokio::spawn;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info, warn};

use crate::core::{InstructEncoderImpl, InstructMatcherImpl, SubmoduleStoreImpl};
use crate::{CANCELLATION_TOKEN, CLOSE_SENDER};

pub fn simple_instruct_manager_thread(
    instruct_encoder: InstructEncoderImpl,
    instruct_matcher: InstructMatcherImpl,
    submodule_store: SubmoduleStoreImpl,
    instruct_receiver: UnboundedReceiver<InstructEntity>,
) -> Result<()> {
    let close_sender = CLOSE_SENDER.get().unwrap().upgrade().unwrap();
    spawn(async move {
        if let Err(e) = start(
            instruct_encoder,
            instruct_matcher,
            submodule_store,
            instruct_receiver,
        )
        .await
        {
            error!("Instruct Manager Thread Error: {}", e);
            CANCELLATION_TOKEN.cancel();
        }
        close_sender
            .send("Instruct Manager Thread".to_string())
            .await
            .unwrap();
    });
    Ok(())
}

async fn start(
    instruct_encoder: InstructEncoderImpl,
    instruct_matcher: InstructMatcherImpl,
    submodule_store: SubmoduleStoreImpl,
    mut instruct_receiver: UnboundedReceiver<InstructEntity>,
) -> Result<()> {
    info!("Instruct Manager Thread Start");
    while let Some(instruct) = instruct_receiver.recv().await {
        info!("Get Instruct：{:?}", &instruct);
        let mut encoded_instruct: Vec<f32> = Vec::new();
        match &instruct.instruct {
            Text(text) => {
                encoded_instruct.append(instruct_encoder.encode(text)?.as_mut());
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
