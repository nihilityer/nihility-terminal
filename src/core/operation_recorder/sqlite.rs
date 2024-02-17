use async_trait::async_trait;
use crate::core::operation_recorder::OperationRecorder;
use anyhow::Result;
use nihility_common::{InstructEntity, ManipulateEntity, ModuleOperate};
use crate::config::OperationRecorderConfig;

#[derive(Default)]
pub struct SqliteOperationRecorder;

#[async_trait]
impl OperationRecorder for SqliteOperationRecorder {
    async fn init(operation_recorder_config: &OperationRecorderConfig) -> Result<Self> where Self: Sized + Send + Sync {
        todo!()
    }

    async fn recorder_instruct(&self, instruct: &InstructEntity) -> Result<()> {
        todo!()
    }

    async fn recorder_manipulate(&self, manipulate: &ManipulateEntity) -> Result<()> {
        todo!()
    }

    async fn recorder_module_operate(&self, module_operate: &ModuleOperate) -> Result<()> {
        todo!()
    }
}