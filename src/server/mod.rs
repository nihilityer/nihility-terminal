use anyhow::Result;
use nihility_common::{GrpcServer, NihilityServer};

use crate::config::ServerConfig;
use crate::{CANCELLATION_TOKEN, INSTRUCT_SENDER, MANIPULATE_SENDER, MODULE_OPERATE_SENDER};

pub async fn server_start(server_config: &ServerConfig) -> Result<()> {
    let instruct_sender = INSTRUCT_SENDER.get().unwrap().upgrade().unwrap();
    let manipulate_sender = MANIPULATE_SENDER.get().unwrap().upgrade().unwrap();
    let submodule_operate_sender = MODULE_OPERATE_SENDER.get().unwrap().upgrade().unwrap();

    let mut grpc_server = GrpcServer::init(
        server_config.grpc_server.clone(),
        CANCELLATION_TOKEN.clone(),
    );
    grpc_server.set_instruct_sender(instruct_sender.clone())?;
    grpc_server.set_manipulate_sender(manipulate_sender.clone())?;
    grpc_server.set_submodule_operate_sender(submodule_operate_sender.clone())?;
    grpc_server.start()?;

    Ok(())
}
