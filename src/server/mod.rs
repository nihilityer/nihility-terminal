use anyhow::Result;
use nihility_common::{GrpcServer, GrpcServerConfig, NihilityServer};

use crate::config::CommunicatConfig;
use crate::{CANCELLATION_TOKEN, INSTRUCT_SENDER, MANIPULATE_SENDER, MODULE_OPERATE_SENDER};

pub async fn server_start(_communicat_config: CommunicatConfig) -> Result<()> {
    let instruct_sender = INSTRUCT_SENDER.get().unwrap().upgrade().unwrap();
    let manipulate_sender = MANIPULATE_SENDER.get().unwrap().upgrade().unwrap();
    let submodule_operate_sender = MODULE_OPERATE_SENDER.get().unwrap().upgrade().unwrap();

    let grpc_server_config = GrpcServerConfig::default();
    let mut grpc_server = GrpcServer::init(grpc_server_config, CANCELLATION_TOKEN.clone());
    grpc_server.set_instruct_sender(instruct_sender.clone())?;
    grpc_server.set_manipulate_sender(manipulate_sender.clone())?;
    grpc_server.set_submodule_operate_sender(submodule_operate_sender.clone())?;
    grpc_server.start()?;

    Ok(())
}
