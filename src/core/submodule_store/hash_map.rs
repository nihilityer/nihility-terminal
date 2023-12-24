use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use anyhow::Result;
use async_trait::async_trait;

use crate::core::submodule_store::SubmoduleStore;
use crate::entity::submodule::Submodule;

#[derive(Default)]
pub struct HashMapSubmoduleStore {
    inner_data: Mutex<HashMap<String, Submodule>>,
}

#[async_trait]
impl SubmoduleStore for HashMapSubmoduleStore {
    async fn init() -> Result<Self>
    where
        Self: Sized + Send + Sync,
    {
        Ok(HashMapSubmoduleStore {
            inner_data: Mutex::new(HashMap::new()),
        })
    }

    async fn insert(&self, submodule: Submodule) -> Result<()> {
        match self.inner_data.lock() {
            Ok(mut data_map) => {
                data_map.insert(submodule.name.to_string(), submodule);
                Ok(())
            }
            Err(e) => Err(anyhow!(
                "HashMapSubmoduleStore Lock Inner Data Error: {}",
                e
            )),
        }
    }

    async fn get_and_remove(&self, name: &String) -> Result<Option<Submodule>> {
        match self.inner_data.lock() {
            Ok(mut data_map) => Ok(data_map.remove(name)),
            Err(e) => Err(anyhow!(
                "HashMapSubmoduleStore Lock Inner Data Error: {}",
                e
            )),
        }
    }

    async fn update_heartbeat(&self, name: &String) -> Result<()> {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        match self.get_and_remove(name).await? {
            None => Err(anyhow!("{} Not In HashMapSubmoduleStore", name)),
            Some(submodule) => {
                match submodule.heartbeat_time.write() {
                    Ok(mut heartbeat_time) => {
                        *heartbeat_time = timestamp;
                    }
                    Err(e) => return Err(anyhow!("Lock {} heartbeat_time, Error: {}", name, e)),
                }
                self.insert(submodule).await?;
                Ok(())
            }
        }
    }

    async fn get_expire_heartbeat_submodule(&self, expire_time: u64) -> Result<Vec<String>> {
        let mut result = Vec::<String>::new();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        match self.inner_data.lock() {
            Ok(data_map) => {
                for (_, submodule) in data_map.iter() {
                    match submodule.heartbeat_time.read() {
                        Ok(heartbeat_time) => {
                            if (timestamp - *heartbeat_time) >= expire_time {
                                result.push(submodule.name.to_string());
                            }
                        }
                        Err(e) => {
                            return Err(anyhow!(
                                "Read Lock {} heartbeat_time Error: {}",
                                &submodule.name,
                                e
                            ))
                        }
                    }
                }
                Ok(result)
            }
            Err(e) => Err(anyhow!(
                "HashMapSubmoduleStore Lock Inner Data Error: {}",
                e
            )),
        }
    }
}
