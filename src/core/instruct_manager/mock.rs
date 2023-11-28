use std::collections::HashMap;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tracing::warn;

use crate::core::instruct_manager::{InstructManager, PointPayload};

#[derive(Default)]
pub struct MockInstructManager;

#[async_trait]
impl InstructManager for MockInstructManager {
    async fn init(config: HashMap<String, String>) -> Result<Self>
    where
        Self: Sized + Send + Sync,
    {
        warn!("Mock Instruct Manager Init Config: {:?}", config);
        return Err(anyhow!("Mock Instruct Manager Cannot Init"));
    }

    async fn search(&self, encode: Vec<f32>) -> Result<String> {
        warn!("Mock Instruct Manager Search encode: {:?}", encode);
        return Err(anyhow!("Mock Instruct Manager Cannot Search"));
    }

    async fn append_points(&self, module_name: String, _points: Vec<PointPayload>) -> Result<()> {
        warn!(
            "Mock Instruct Manager Append Points, module_name: {:?}",
            module_name
        );
        return Err(anyhow!("Mock Instruct Manager Cannot Append Points"));
    }

    async fn remove_points(&self, _points: Vec<String>) -> Result<()> {
        warn!("Mock Instruct Manager Remove Points");
        return Err(anyhow!("Mock Instruct Manager Cannot Remove Points"));
    }
}
