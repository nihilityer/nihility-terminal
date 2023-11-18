use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::config::{ManagerType, ModuleManagerConfig};
use crate::core::encoder::Encoder;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::ModuleOperate;
use crate::AppError;

mod grpc_qdrant;

/// 所有子模块管理模块都需要实现此特征
#[async_trait]
pub trait ModuleManager {
    /// 只需要启动即可
    async fn start(
        config: HashMap<String, String>,
        encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
        module_operate_sender: Sender<ModuleOperate>,
        module_operate_receiver: Receiver<ModuleOperate>,
        instruct_receiver: Receiver<InstructEntity>,
        manipulate_receiver: Receiver<ManipulateEntity>,
    ) -> Result<(), AppError>;
}

pub async fn module_manager_builder(
    module_manager_config: &ModuleManagerConfig,
    encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
    module_operate_sender: Sender<ModuleOperate>,
    module_operate_receiver: Receiver<ModuleOperate>,
    instruct_receiver: Receiver<InstructEntity>,
    manipulate_receiver: Receiver<ManipulateEntity>,
) -> Result<(), AppError> {
    tracing::info!(
        "Module Manager Type: {:?}",
        &module_manager_config.manager_type
    );
    return match &module_manager_config.manager_type {
        ManagerType::GrpcQdrant => {
            Ok(grpc_qdrant::GrpcQdrant::start(
                module_manager_config.config_map.clone(),
                encoder,
                module_operate_sender,
                module_operate_receiver,
                instruct_receiver,
                manipulate_receiver,
            )
            .await?)
        }
    };
}
