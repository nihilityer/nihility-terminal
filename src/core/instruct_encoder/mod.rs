use anyhow::Result;

use crate::config::InstructEncoderConfig;

pub mod sentence_transformers;

pub trait InstructEncoder {
    fn init(instruct_encoder_config: &InstructEncoderConfig) -> Result<Self>
    where
        Self: Sized + Send + Sync;

    fn encode(&self, input: &str) -> Result<Vec<f32>>;

    fn encode_size(&self) -> u64;
}
