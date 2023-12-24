use std::collections::HashMap;
use std::sync::{Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use nihility_common::{
    ClientType, ConnectionType, GrpcClient, GrpcClientConfig, ModuleOperate, NihilityClient,
    OperateType,
};

pub struct Submodule {
    pub name: String,
    pub default_instruct_map: Mutex<HashMap<String, String>>,
    pub connection_type: ConnectionType,
    pub client_type: ClientType,
    pub heartbeat_time: RwLock<u64>,
    pub client: Box<dyn NihilityClient + Send + Sync>,
}

impl Submodule {
    pub async fn create(module_operate: &ModuleOperate) -> Result<Self> {
        if let OperateType::Register = &module_operate.operate_type {
            if let Some(info) = &module_operate.info {
                match info.connection_type {
                    ConnectionType::GrpcType => {
                        let grpc_client_config =
                            GrpcClientConfig::try_from(info.conn_params.clone())?;
                        let mut client = GrpcClient::init(grpc_client_config);
                        let client_type = match info.client_type {
                            ClientType::BothType => {
                                client.connection_instruct_server().await?;
                                client.connection_manipulate_server().await?;
                                ClientType::BothType
                            }
                            ClientType::InstructType => {
                                client.connection_instruct_server().await?;
                                ClientType::InstructType
                            }
                            ClientType::ManipulateType => {
                                client.connection_manipulate_server().await?;
                                ClientType::ManipulateType
                            }
                        };
                        let mut default_instruct_map = HashMap::<String, String>::new();
                        for instruct in &info.default_instruct {
                            default_instruct_map.insert(instruct.to_string(), String::new());
                        }
                        return Ok(Submodule {
                            name: module_operate.name.to_string(),
                            default_instruct_map: Mutex::new(default_instruct_map),
                            connection_type: ConnectionType::GrpcType,
                            client_type,
                            heartbeat_time: RwLock::new(
                                SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
                            ),
                            client: Box::new(client),
                        });
                    }
                    ConnectionType::PipeType => {
                        return Err(anyhow!("ModuleOperate PipeType Not Support Yet"))
                    }
                    ConnectionType::WindowsNamedPipeType => {
                        return Err(anyhow!(
                            "ModuleOperate WindowsNamedPipeType Not Support Yet"
                        ))
                    }
                    ConnectionType::HttpType => {
                        return Err(anyhow!("ModuleOperate HttpType Not Support Yet"))
                    }
                }
            }
        }
        Err(anyhow!("ModuleOperate OperateType Error"))
    }
}
