use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use anyhow::Result;
use async_trait::async_trait;

use crate::config::SubmoduleStoreConfig;
use crate::core::submodule_store::SubmoduleStore;
use crate::entity::submodule::Submodule;

#[derive(Default)]
pub struct HashMapSubmoduleStore {
    inner_data: HashMap<String, Submodule>,
}

#[async_trait]
impl SubmoduleStore for HashMapSubmoduleStore {
    async fn init(_submodule_store_config: &SubmoduleStoreConfig) -> Result<Self>
    where
        Self: Sized + Send + Sync,
    {
        Ok(HashMapSubmoduleStore {
            inner_data: HashMap::new(),
        })
    }

    async fn insert(&mut self, submodule: Submodule) -> Result<()> {
        self.inner_data
            .insert(submodule.name.to_string(), submodule);
        Ok(())
    }

    async fn get(&self, name: &String) -> Result<Option<&Submodule>> {
        Ok(self.inner_data.get(name))
    }

    async fn get_mut(&mut self, name: &String) -> Result<Option<&mut Submodule>> {
        Ok(self.inner_data.get_mut(name))
    }

    async fn update_heartbeat(&mut self, name: &String) -> Result<()> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        match self.inner_data.get_mut(name) {
            None => Err(anyhow!("{} Not In HashMapSubmoduleStore", name)),
            Some(submodule) => {
                submodule.heartbeat_time = timestamp;
                Ok(())
            }
        }
    }

    async fn get_expire_heartbeat_submodule(&self, expire_time: u64) -> Result<Vec<String>> {
        let mut result = Vec::<String>::new();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        for (_, submodule) in self.inner_data.iter() {
            if (timestamp - submodule.heartbeat_time) >= expire_time {
                result.push(submodule.name.to_string());
            }
        }
        Ok(result)
    }

    async fn remove_submodule(&mut self, name: &String) -> Result<Submodule> {
        match self.inner_data.remove(name) {
            None => Err(anyhow!("Cannot Find Named {} Submodule", name)),
            Some(submodule) => Ok(submodule),
        }
    }
}
