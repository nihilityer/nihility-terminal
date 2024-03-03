use anyhow::Result;
use nihility_common::ModuleOperate;
use tokio::spawn;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::error;

pub use simple::simple_submodule_manager_thread;

use crate::core::{
    InstructEncoderImpl, InstructMatcherImpl, OperationRecorderImpl, SubmoduleManagerFn,
    SubmoduleStoreImpl,
};
use crate::{CANCELLATION_TOKEN, CLOSE_SENDER};

mod simple;

pub fn submodule_manager_thread(
    submodule_manager_fn: Box<SubmoduleManagerFn>,
    instruct_encoder: InstructEncoderImpl,
    instruct_matcher: InstructMatcherImpl,
    submodule_store: SubmoduleStoreImpl,
    operation_recorder: OperationRecorderImpl,
    module_operate_receiver: UnboundedReceiver<ModuleOperate>,
) -> Result<()> {
    let close_sender = CLOSE_SENDER.get().unwrap().upgrade().unwrap();
    spawn(async move {
        if let Err(e) = submodule_manager_fn(
            instruct_encoder,
            instruct_matcher,
            submodule_store,
            operation_recorder,
            module_operate_receiver,
        )
        .await
        {
            error!("Submodule Manager Thread Error: {}", e);
            CANCELLATION_TOKEN.cancel();
        }
        close_sender
            .send("Submodule Manager Thread".to_string())
            .await
            .unwrap();
    });
    Ok(())
}
