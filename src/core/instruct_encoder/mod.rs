use anyhow::Result;
use async_trait::async_trait;

use crate::config::InstructEncoderConfig;

pub mod sentence_transformers;

#[async_trait]
pub trait InstructEncoder {
    async fn init(instruct_encoder_config: &InstructEncoderConfig) -> Result<Self>
    where
        Self: Sized + Send + Sync;

    async fn encode(&self, input: &str) -> Result<Vec<f32>>;

    async fn encode_size(&self) -> u64;
}
