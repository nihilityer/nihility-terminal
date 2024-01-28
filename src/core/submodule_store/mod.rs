use anyhow::Result;
use async_trait::async_trait;

use crate::entity::submodule::Submodule;

mod hash_map;

use crate::config::SubmoduleStoreConfig;
pub use hash_map::HashMapSubmoduleStore;

#[async_trait]
pub trait SubmoduleStore {
    async fn init(submodule_store_config: &SubmoduleStoreConfig) -> Result<Self>
    where
        Self: Sized + Send + Sync;
    async fn insert(&mut self, submodule: Submodule) -> Result<()>;
    async fn get(&self, name: &String) -> Result<Option<&Submodule>>;
    async fn get_mut(&mut self, name: &String) -> Result<Option<&mut Submodule>>;
    async fn update_heartbeat(&mut self, name: &String) -> Result<()>;
    async fn get_expire_heartbeat_submodule(&self, expire_time: u64) -> Result<Vec<String>>;
    async fn remove_submodule(&mut self, name: &String) -> Result<Submodule>;
}
