use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::config::{InstructManagerConfig, InstructManagerType};
use crate::AppError;

mod grpc_qdrant;

pub const ENCODE_SIZE_FIELD: &str = "encode_size";

pub struct PointPayload {
    pub encode: Vec<f32>,
    pub instruct: String,
    pub uuid: String,
}

/// 所有子模块管理模块都需要实现此特征
#[async_trait]
pub trait InstructManager {
    async fn init(config: HashMap<String, String>) -> Result<Self, AppError>
    where
        Self: Sized;

    async fn search(&self, encode: Vec<f32>) -> Result<String, AppError>;

    async fn append_points(
        &self,
        module_name: String,
        points: Vec<PointPayload>,
    ) -> Result<(), AppError>;

    async fn remove_points(&self, points: Vec<String>) -> Result<(), AppError>;
}

pub async fn build_instruct_manager(
    config: InstructManagerConfig,
) -> Result<Arc<tokio::sync::Mutex<Box<dyn InstructManager + Send>>>, AppError> {
    match config.manager_type {
        InstructManagerType::GrpcQdrant => {
            let instruct_manager = grpc_qdrant::GrpcQdrant::init(config.config_map.clone()).await?;
            Ok(Arc::new(tokio::sync::Mutex::new(Box::new(
                instruct_manager,
            ))))
        }
    }
}
