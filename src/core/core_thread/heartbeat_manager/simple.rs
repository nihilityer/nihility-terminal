use std::time::Duration;

use anyhow::Result;
use nihility_common::{ModuleOperate, OperateType};
use tracing::{debug, info};

use crate::core::core_thread::heartbeat_manager::HEARTBEAT_TIME;
use crate::core::SubmoduleStoreImpl;
use crate::MODULE_OPERATE_SENDER;

pub async fn simple_heartbeat_manager_thread(submodule_store: SubmoduleStoreImpl) -> Result<()> {
    info!("Heartbeat Manager Thread Start");
    let mut interval = tokio::time::interval(Duration::from_secs(HEARTBEAT_TIME));
    loop {
        interval.tick().await;
        debug!("Check Submodule Heartbeat");
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
