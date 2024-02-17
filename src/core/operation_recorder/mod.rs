use anyhow::Result;
use async_trait::async_trait;
use nihility_common::{InstructEntity, ManipulateEntity, ModuleOperate};

pub use log::LogOperationRecorder;
pub use sqlite::SqliteOperationRecorder;

use crate::config::OperationRecorderConfig;

mod log;
mod sqlite;

#[async_trait]
pub trait OperationRecorder {
    async fn init(operation_recorder_config: &OperationRecorderConfig) -> Result<Self>
    where
        Self: Sized + Send + Sync;
    async fn recorder_instruct(&self, instruct: &InstructEntity) -> Result<()>;
    async fn recorder_manipulate(&self, manipulate: &ManipulateEntity) -> Result<()>;
    async fn recorder_module_operate(&self, module_operate: &ModuleOperate) -> Result<()>;
}
