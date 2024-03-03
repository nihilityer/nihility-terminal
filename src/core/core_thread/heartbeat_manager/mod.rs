use anyhow::Result;
use tokio::{select, spawn};
use tracing::error;

pub use simple::simple_heartbeat_manager_thread;

use crate::core::{HeartbeatManagerFn, SubmoduleStoreImpl};
use crate::{CANCELLATION_TOKEN, CLOSE_SENDER};

mod simple;

static HEARTBEAT_TIME: u64 = 30;

pub fn heartbeat_manager_thread(
    heartbeat_manager_fn: Box<HeartbeatManagerFn>,
    submodule_store: SubmoduleStoreImpl,
) -> Result<()> {
    let close_sender = CLOSE_SENDER.get().unwrap().upgrade().unwrap();
    spawn(async move {
        select! {
            heartbeat_result = heartbeat_manager_fn(submodule_store) => {
                if let Err(e) = heartbeat_result {
                    error!("Heartbeat Manager Thread Error: {}", e);
                    CANCELLATION_TOKEN.cancel();
                }
            },
            _ = CANCELLATION_TOKEN.cancelled() => {},
        }
        close_sender
            .send("Heartbeat Manager Thread".to_string())
            .await
            .unwrap();
    });
    Ok(())
}
