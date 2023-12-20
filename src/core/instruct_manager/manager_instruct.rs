use anyhow::{anyhow, Result};
use nihility_common::InstructEntity;
use tokio::spawn;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::{debug, error, info, warn};

use crate::CANCELLATION_TOKEN;


pub(super) fn start(
    shutdown_sender: UnboundedSender<String>,
    instruct_receiver: UnboundedReceiver<InstructEntity>,
) {
    spawn(async move {
        if let Err(e) = manager_instruct(instruct_receiver).await {
            error!("Instruct Manager Error: {}", e);
            CANCELLATION_TOKEN.cancel();
        }
        shutdown_sender
            .send("Instruct Manager".to_string())
            .unwrap();
    });
}

/// 处理指令的接收和转发
///
/// 1、通过模块索引找到处理对应指令的模块，将指令转发
///
/// 2、记录操作到日志
///
/// 3、转发指令出现错误时选择性重试
pub(super) async fn manager_instruct(
    mut instruct_receiver: UnboundedReceiver<InstructEntity>,
) -> Result<()> {
    info!("Start Receive Instruct");
    while let Some(instruct) = instruct_receiver.recv().await {
        info!("Get Instruct：{:?}", &instruct);
        let mut encoded_instruct: Vec<f32> = Vec::new();
        if let Ok(mut encoder) = INSTRUCT_ENCODER.get().unwrap().lock() {
            if let Text(text) = instruct.instruct.clone() {
                encoded_instruct.append(encoder.encode(text)?.as_mut());
            } else {
                error!("Cannot Forward This Type Instruct");
                continue;
            }
        } else {
            return Err(anyhow!("Lock Instruct Encoder Error"));
        }

        let locked_instruct_manager = INSTRUCT_MANAGER.get().unwrap().lock().await;
        match locked_instruct_manager.search(encoded_instruct).await {
            Ok(module_name) => {
                drop(locked_instruct_manager);

                let mut modules = SUBMODULE_MAP.lock().await;
                if let Some(module) = modules.get_mut(module_name.as_str()) {
                    match module.send_instruct(instruct).await {
                        Ok(RespCode::Success) => {
                            debug!("Forward Instruct Success");
                        }
                        Ok(other_resp_code) => {
                            error!("Forward Instruct Fail, Resp Code: {:?}", other_resp_code);
                        }
                        Err(e) => {
                            error!("Forward Instruct Error: {}", e);
                        }
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
