use anyhow::Result;
use nihility_common::{
    GrpcServer, GrpcServerConfig, InstructEntity, ManipulateEntity, ModuleOperate, NihilityServer,
};
use tokio::sync::mpsc::UnboundedSender;
use crate::CANCELLATION_TOKEN;

use crate::config::CommunicatConfig;

pub async fn server_start(
    _communicat_config: CommunicatConfig,
    instruct_sender: UnboundedSender<InstructEntity>,
    manipulate_sender: UnboundedSender<ManipulateEntity>,
    submodule_operate_sender: UnboundedSender<ModuleOperate>,
) -> Result<()> {
    let grpc_server_config = GrpcServerConfig::default();
    let mut grpc_server = GrpcServer::init(grpc_server_config, CANCELLATION_TOKEN.clone());
    grpc_server.set_instruct_sender(instruct_sender.clone())?;
    grpc_server.set_manipulate_sender(manipulate_sender.clone())?;
    grpc_server.set_submodule_operate_sender(submodule_operate_sender.clone())?;
    grpc_server.start()?;

    Ok(())
}
