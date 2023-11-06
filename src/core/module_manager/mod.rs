use crate::config::ModuleManagerConfig;
use crate::core::encoder::Encoder;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::Module;
use crate::AppError;
use async_trait::async_trait;
use std::sync::{Arc};
use tokio::sync::mpsc::Receiver;
use std::sync::Mutex;

mod grpc_qrdant;

/// 所有子模块管理模块都需要实现此特征
#[async_trait]
pub trait ModuleManager {
    /// 只需要启动即可
    async fn start(
        encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
        module_receiver: Receiver<Module>,
        instruct_receiver: Receiver<InstructEntity>,
        manipulate_receiver: Receiver<ManipulateEntity>,
    ) -> Result<(), AppError>;
}

pub async fn module_manager_builder(
    module_manager_config: &ModuleManagerConfig,
    encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
    module_receiver: Receiver<Module>,
    instruct_receiver: Receiver<InstructEntity>,
    manipulate_receiver: Receiver<ManipulateEntity>,
) -> Result<(), AppError> {
    tracing::info!("Module Manager Type: {}", &module_manager_config.manager_type);
    return match module_manager_config.manager_type.to_lowercase().as_str() {
        "grpc_qrdant" => Ok(grpc_qrdant::GrpcQdrant::start(
            encoder,
            module_receiver,
            instruct_receiver,
            manipulate_receiver,
        )
        .await?),
        _ => Err(AppError::ModuleManagerError(
            "not support manager type".to_string(),
        )),
    };
}
