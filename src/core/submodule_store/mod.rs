use crate::entity::submodule::Submodule;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait SubmoduleStore: Clone + Default {
    async fn insert(&self, submodule: Submodule) -> Result<()>;
}