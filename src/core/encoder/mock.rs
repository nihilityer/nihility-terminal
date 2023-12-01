use anyhow::{anyhow, Result};
use tracing::{debug, warn};

use crate::core::encoder::InstructEncoder;

#[derive(Default)]
pub struct MockInstructEncoder;

impl InstructEncoder for MockInstructEncoder {
    fn init(model_path: String, model_name: String) -> Result<Self>
    where
        Self: Sized + Send + Sync,
    {
        debug!(
            "Mock Instruct Encoder Init Params: {:?}, {:?}",
            model_path, model_name
        );
        Ok(MockInstructEncoder::default())
    }

    fn encode(&mut self, input: String) -> Result<Vec<f32>> {
        warn!("Use Mock Instruct Encoder to encode, input: {:?}", input);
        Err(anyhow!("This Instruct Encoder Cannot Encode"))
    }

    fn encode_size(&self) -> u64 {
        return 0;
    }
}
