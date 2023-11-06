use nihility_common::instruct::instruct_server::InstructServer;
use nihility_common::manipulate::manipulate_server::ManipulateServer;
use nihility_common::module_info::module_info_server::ModuleInfoServer;
use tokio::sync::mpsc::Sender;
use tonic::transport::Server;

use crate::AppError;
use crate::communicat::grpc::{InstructImpl, ManipulateImpl, ModuleInfoImpl};
use crate::config::GrpcConfig;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::Module;

pub struct GrpcServer;

impl GrpcServer {
    pub async fn start(
        grpc_config: &GrpcConfig,
        module_sender: Sender<Module>,
        instruct_sender: Sender<InstructEntity>,
        manipulate_sender: Sender<ManipulateEntity>,
    ) -> Result<(), AppError> {
        if grpc_config.enable {
            tracing::info!("GrpcServer start!");
            let mut grpc_addr = grpc_config.addr.to_string();
            grpc_addr.push_str(format!(":{}", grpc_config.port).as_str());

            Server::builder()
                .add_service(ModuleInfoServer::new(ModuleInfoImpl::init(module_sender)))
                .add_service(InstructServer::new(InstructImpl::init(instruct_sender)))
                .add_service(ManipulateServer::new(ManipulateImpl::init(
                    manipulate_sender,
                )))
                .serve(grpc_addr.parse()?)
                .await?;
        }

        Ok(())
    }
}
