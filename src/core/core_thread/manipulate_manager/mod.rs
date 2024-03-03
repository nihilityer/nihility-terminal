use anyhow::Result;
use nihility_common::ManipulateEntity;
use tokio::spawn;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::error;

pub use simple::simple_manipulate_manager_thread;

use crate::core::{ManipulateManagerFn, OperationRecorderImpl, SubmoduleStoreImpl};
use crate::{CANCELLATION_TOKEN, CLOSE_SENDER};

mod simple;

pub fn manipulate_manager_thread(
    manipulate_manager_fn: Box<ManipulateManagerFn>,
    submodule_store: SubmoduleStoreImpl,
    operation_recorder: OperationRecorderImpl,
    manipulate_receiver: UnboundedReceiver<ManipulateEntity>,
) -> Result<()> {
    let close_sender = CLOSE_SENDER.get().unwrap().upgrade().unwrap();
    spawn(async move {
        if let Err(e) =
            manipulate_manager_fn(submodule_store, operation_recorder, manipulate_receiver).await
        {
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
