use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use nihility_common::{
    ClientType, ConnectionType, GrpcClient, GrpcClientConfig, ModuleOperate, NihilityClient,
    OperateType,
};
use tracing::debug;

pub struct Submodule {
    pub name: String,
    pub default_instruct_map: HashMap<String, String>,
    pub connection_type: ConnectionType,
    pub client_type: ClientType,
    pub heartbeat_time: u64,
    pub client: Box<dyn NihilityClient + Send + Sync>,
}

impl Submodule {
    pub async fn create(module_operate: &ModuleOperate) -> Result<Self> {
        debug!("Create Submodule Use Module Operate: {:?}", &module_operate);
        if let OperateType::Register = &module_operate.operate_type {
            if let Some(info) = &module_operate.info {
                return match &info.conn_params.connection_type {
                    ConnectionType::GrpcType => {
                        let (client, client_type) = match &info.conn_params.client_type {
                            ClientType::BothType => {
                                let grpc_client_config = GrpcClientConfig::try_from(
                                    info.conn_params.conn_config.clone(),
                                )?;
                                let mut client = GrpcClient::init(grpc_client_config);
                                client.connection_instruct_server().await?;
                                client.connection_manipulate_server().await?;
                                (client, ClientType::BothType)
                            }
                            ClientType::InstructType => {
                                let grpc_client_config = GrpcClientConfig::try_from(
                                    info.conn_params.conn_config.clone(),
                                )?;
                                let mut client = GrpcClient::init(grpc_client_config);
                                client.connection_instruct_server().await?;
                                (client, ClientType::InstructType)
                            }
                            ClientType::ManipulateType => {
                                let grpc_client_config = GrpcClientConfig::try_from(
                                    info.conn_params.conn_config.clone(),
                                )?;
                                let mut client = GrpcClient::init(grpc_client_config);
                                client.connection_manipulate_server().await?;
                                (client, ClientType::ManipulateType)
                            }
                            ClientType::NotReceiveType => {
                                (GrpcClient::init(GrpcClientConfig::default()), ClientType::ManipulateType)
                            }
                        };
                        let mut default_instruct_map = HashMap::<String, String>::new();
                        for instruct in &info.default_instruct {
                            default_instruct_map.insert(instruct.to_string(), String::new());
                        }
                        Ok(Submodule {
                            name: module_operate.name.to_string(),
                            default_instruct_map,
                            connection_type: ConnectionType::GrpcType,
                            client_type,
                            heartbeat_time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
                            client: Box::new(client),
                        })
                    }
                    ConnectionType::PipeType => {
                        Err(anyhow!("ModuleOperate PipeType Not Support Yet"))
                    }
                    ConnectionType::WindowsNamedPipeType => Err(anyhow!(
                        "ModuleOperate WindowsNamedPipeType Not Support Yet"
                    )),
                    ConnectionType::HttpType => {
                        Err(anyhow!("ModuleOperate HttpType Not Support Yet"))
                    }
                };
            }
        }
        Err(anyhow!("ModuleOperate OperateType Error"))
    }
}
