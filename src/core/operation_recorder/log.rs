use anyhow::Result;
use async_trait::async_trait;
use nihility_common::{InstructEntity, ManipulateEntity, ModuleOperate};
use tracing::info;

use crate::config::OperationRecorderConfig;
use crate::core::operation_recorder::OperationRecorder;

#[derive(Default)]
pub struct LogOperationRecorder;

#[async_trait]
impl OperationRecorder for LogOperationRecorder {
    async fn init(_operation_recorder_config: &OperationRecorderConfig) -> Result<Self>
    where
        Self: Sized + Send + Sync,
    {
        Ok(LogOperationRecorder)
    }

    async fn recorder_instruct(&self, instruct: &InstructEntity) -> Result<()> {
        info!("Recorder Instruct: {:?}", instruct);
        Ok(())
    }

    async fn recorder_manipulate(&self, manipulate: &ManipulateEntity) -> Result<()> {
        info!("Recorder Manipulate: {:?}", manipulate);
        Ok(())
    }

    async fn recorder_module_operate(&self, module_operate: &ModuleOperate) -> Result<()> {
        info!("Recorder ModuleOperate: {:?}", module_operate);
        Ok(())
    }
}
