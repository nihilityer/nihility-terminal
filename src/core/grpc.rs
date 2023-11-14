use nihility_common::instruct::instruct_server::InstructServer;
use nihility_common::manipulate::manipulate_server::ManipulateServer;
use nihility_common::sub_module::sub_module_server::SubModuleServer;
use tokio::sync::mpsc::Sender;
use tonic::transport::Server;

use crate::communicat::grpc::{InstructImpl, ManipulateImpl, SubModuleImpl};
use crate::config::GrpcConfig;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::ModuleOperate;
use crate::AppError;

pub struct GrpcServer;

impl GrpcServer {
    pub async fn start(
        grpc_config: &GrpcConfig,
        operate_module_sender: Sender<ModuleOperate>,
        instruct_sender: Sender<InstructEntity>,
        manipulate_sender: Sender<ManipulateEntity>,
    ) -> Result<(), AppError> {
        if grpc_config.enable {
            let bind_addr = format!(
                "{}:{}",
                grpc_config.addr.to_string(),
                grpc_config.port.to_string()
            );
            tracing::info!("Grpc Server bind at {}", &bind_addr);

            Server::builder()
                .add_service(SubModuleServer::new(SubModuleImpl::init(
                    operate_module_sender,
                )))
                .add_service(InstructServer::new(InstructImpl::init(instruct_sender)))
                .add_service(ManipulateServer::new(ManipulateImpl::init(
                    manipulate_sender,
                )))
                .serve(bind_addr.parse()?)
                .await?;
        }

        Ok(())
    }
}
