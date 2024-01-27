use std::time::Duration;

use anyhow::Result;
use nihility_common::{ModuleOperate, OperateType};
use tokio::{select, spawn};
use tracing::{debug, error, info};

use crate::core::core_thread::heartbeat_manager::HEARTBEAT_TIME;
use crate::core::SubmoduleStoreImpl;
use crate::{CANCELLATION_TOKEN, CLOSE_SENDER, MODULE_OPERATE_SENDER};

pub fn simple_heartbeat_manager_thread(submodule_store: SubmoduleStoreImpl) -> Result<()> {
    let close_sender = CLOSE_SENDER.get().unwrap().upgrade().unwrap();
    spawn(async move {
        select! {
            heartbeat_result = start(submodule_store) => {
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

async fn start(submodule_store: SubmoduleStoreImpl) -> Result<()> {
    info!("Heartbeat Manager Thread Start");
    let mut interval = tokio::time::interval(Duration::from_secs(HEARTBEAT_TIME));
    loop {
        interval.tick().await;
        debug!("Make Sure The Submodule Heartbeat Is Normal");
        for name in submodule_store
            .lock()
            .await
            .get_expire_heartbeat_submodule(HEARTBEAT_TIME * 2)
            .await?
        {
            info!("Submodule {:?} Heartbeat Exception", &name);
            let mut operate = ModuleOperate::default();
            operate.name = name;
            operate.operate_type = OperateType::Offline;
            MODULE_OPERATE_SENDER
                .get()
                .unwrap()
                .upgrade()
                .unwrap()
                .send(operate)?;
        }
    }
}
