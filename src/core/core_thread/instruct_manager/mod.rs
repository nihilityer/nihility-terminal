use anyhow::Result;
use nihility_common::InstructEntity;
use tokio::spawn;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::error;

pub use simple::simple_instruct_manager_thread;

use crate::core::{
    InstructEncoderImpl, InstructManagerFn, InstructMatcherImpl, OperationRecorderImpl,
    SubmoduleStoreImpl,
};
use crate::{CANCELLATION_TOKEN, CLOSE_SENDER};

mod simple;

pub fn instruct_manager_thread(
    instruct_manager_fn: Box<InstructManagerFn>,
    instruct_encoder: InstructEncoderImpl,
    instruct_matcher: InstructMatcherImpl,
    submodule_store: SubmoduleStoreImpl,
    operation_recorder: OperationRecorderImpl,
    instruct_receiver: UnboundedReceiver<InstructEntity>,
) -> Result<()> {
    let close_sender = CLOSE_SENDER.get().unwrap().upgrade().unwrap();
    spawn(async move {
        if let Err(e) = instruct_manager_fn(
            instruct_encoder,
            instruct_matcher,
            submodule_store,
            operation_recorder,
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
