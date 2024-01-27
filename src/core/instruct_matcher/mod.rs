use anyhow::Result;
use async_trait::async_trait;

use crate::config::InstructMatcherConfig;

pub mod grpc_qdrant;
pub mod instant_distance;

pub const ENCODE_SIZE_FIELD: &str = "encode_size";

#[derive(Clone, Default, Debug)]
pub struct PointPayload {
    pub encode: Vec<f32>,
    pub submodule_id: String,
    pub instruct: String,
    pub uuid: String,
}

impl PartialEq for PointPayload {
    fn eq(&self, other: &Self) -> bool {
        self.encode.eq(&other.encode)
    }
}

#[async_trait]
pub trait InstructMatcher {
    async fn init(instruct_matcher_config: &InstructMatcherConfig) -> Result<Self>
    where
        Self: Sized + Send + Sync;

    async fn search(&self, point: Vec<f32>) -> Result<String>;

    async fn append_points(&mut self, points: Vec<PointPayload>) -> Result<()>;

    async fn remove_points(&mut self, points: Vec<PointPayload>) -> Result<()>;
}
