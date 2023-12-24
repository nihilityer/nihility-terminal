use anyhow::Result;
use async_trait::async_trait;

use crate::entity::submodule::Submodule;

mod hash_map;

pub use hash_map::HashMapSubmoduleStore;

#[async_trait]
pub trait SubmoduleStore {
    async fn init() -> Result<Self> where Self: Sized + Send + Sync;
    async fn insert(&self, submodule: Submodule) -> Result<()>;
    async fn get_and_remove(&self, name: &String) -> Result<Option<Submodule>>;
    async fn update_heartbeat(&self, name: &String) -> Result<()>;
    async fn get_expire_heartbeat_submodule(&self, expire_time: u64) -> Result<Vec<String>>;
}
