use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;

pub mod grpc_qdrant;

pub const ENCODE_SIZE_FIELD: &str = "encode_size";

pub struct PointPayload {
    pub encode: Vec<f32>,
    pub instruct: String,
    pub uuid: String,
}

/// 所有子模块管理模块都需要实现此特征
#[async_trait]
pub trait InstructMatcher {
    /// 初始化指令管理组件
    async fn init(config: HashMap<String, String>) -> Result<Self>
    where
        Self: Sized + Send + Sync;

    /// 从指令管理组件中搜索匹配的子模块名称
    async fn search(&self, encode: Vec<f32>) -> Result<String>;

    /// 批量加入子模块默认指令编码结果向量点
    async fn append_points(&self, module_name: String, points: Vec<PointPayload>) -> Result<()>;

    /// 批量移除子模块默认指令编码结果向量点
    async fn remove_points(&self, points: Vec<String>) -> Result<()>;
}
