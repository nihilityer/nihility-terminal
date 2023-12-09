use std::pin::Pin;

use anyhow::Result;
use nihility_common::instruct::instruct_server::InstructServer;
use nihility_common::manipulate::manipulate_server::ManipulateServer;
use nihility_common::response_code::Resp;
use nihility_common::submodule::submodule_server::SubmoduleServer;
use tokio::sync::mpsc::UnboundedSender;
use tonic::codegen::tokio_stream::Stream;
use tonic::transport::Server;
use tonic::Status;
use tracing::info;

use crate::communicat::grpc::server::instruct::InstructImpl;
use crate::communicat::grpc::server::manipulate::ManipulateImpl;
use crate::communicat::grpc::server::submodule::SubmoduleImpl;
use crate::config::GrpcConfig;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::submodule::ModuleOperate;
use crate::CANCELLATION_TOKEN;

mod instruct;
mod manipulate;
mod submodule;

pub(self) type StreamResp = Pin<Box<dyn Stream<Item = Result<Resp, Status>> + Send>>;

pub(super) async fn start_server(
    grpc_config: GrpcConfig,
    operate_module_sender: UnboundedSender<ModuleOperate>,
    instruct_sender: UnboundedSender<InstructEntity>,
    manipulate_sender: UnboundedSender<ManipulateEntity>,
) -> Result<()> {
    if !grpc_config.enable {
        return Ok(());
    }
    let bind_addr = format!("{}:{}", grpc_config.addr, grpc_config.port);
    info!("Grpc Server Bind At {}", &bind_addr);

    Server::builder()
        .add_service(SubmoduleServer::new(SubmoduleImpl::init(
            operate_module_sender,
        )))
        .add_service(InstructServer::new(InstructImpl::init(instruct_sender)))
        .add_service(ManipulateServer::new(ManipulateImpl::init(
            manipulate_sender,
        )))
        .serve_with_shutdown(bind_addr.parse()?, async move {
            CANCELLATION_TOKEN.cancelled().await
        })
        .await?;

    Ok(())
}
