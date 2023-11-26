use anyhow::Result;
use async_trait::async_trait;
use nihility_common::instruct::instruct_client::InstructClient;
use nihility_common::instruct::instruct_server::{Instruct, InstructServer};
use nihility_common::instruct::{InstructReq, InstructResp};
use nihility_common::manipulate::manipulate_client::ManipulateClient;
use nihility_common::manipulate::manipulate_server::{Manipulate, ManipulateServer};
use nihility_common::manipulate::{ManipulateReq, ManipulateResp};
use nihility_common::response_code::RespCode;
use nihility_common::submodule::submodule_server::{Submodule, SubmoduleServer};
use nihility_common::submodule::{SubModuleResp, SubmoduleReq};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use tonic::transport::{Channel, Server};
use tonic::{Request, Response, Status};
use tracing::info;

use crate::communicat::{SendInstructOperate, SendManipulateOperate};
use crate::config::GrpcConfig;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::{ModuleOperate, OperateType};

pub struct GrpcServer;

impl GrpcServer {
    pub async fn start(
        grpc_config: GrpcConfig,
        cancellation_token: CancellationToken,
        operate_module_sender: UnboundedSender<ModuleOperate>,
        instruct_sender: UnboundedSender<InstructEntity>,
        manipulate_sender: UnboundedSender<ManipulateEntity>,
    ) -> Result<()> {
        if !grpc_config.enable {
            return Ok(());
        }
        let bind_addr = format!(
            "{}:{}",
            grpc_config.addr.to_string(),
            grpc_config.port.to_string()
        );
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
                cancellation_token.cancelled().await
            })
            .await?;

        Ok(())
    }
}

#[async_trait]
impl SendInstructOperate for InstructClient<Channel> {
    async fn send(&mut self, instruct: InstructReq) -> Result<RespCode> {
        Ok(self
            .send_instruct(Request::new(instruct))
            .await?
            .into_inner()
            .resp_code())
    }
}

#[async_trait]
impl SendManipulateOperate for ManipulateClient<Channel> {
    async fn send(&mut self, manipulate: ManipulateReq) -> Result<RespCode> {
        Ok(self
            .send_manipulate(Request::new(manipulate))
            .await?
            .into_inner()
            .resp_code())
    }
}

pub struct SubmoduleImpl {
    operate_module_sender: UnboundedSender<ModuleOperate>,
}

pub struct InstructImpl {
    instruct_sender: UnboundedSender<InstructEntity>,
}

pub struct ManipulateImpl {
    manipulate_sender: UnboundedSender<ManipulateEntity>,
}

#[tonic::async_trait]
impl Submodule for SubmoduleImpl {
    async fn register(
        &self,
        request: Request<SubmoduleReq>,
    ) -> Result<Response<SubModuleResp>, Status> {
        self.operate_module_sender
            .send(ModuleOperate::create_by_req(
                request.into_inner(),
                OperateType::REGISTER,
            ))
            .unwrap();
        Ok(Response::new(SubModuleResp {
            success: true,
            resp_code: RespCode::Success.into(),
        }))
    }

    async fn offline(
        &self,
        request: Request<SubmoduleReq>,
    ) -> Result<Response<SubModuleResp>, Status> {
        self.operate_module_sender
            .send(ModuleOperate::create_by_req(
                request.into_inner(),
                OperateType::OFFLINE,
            ))
            .unwrap();
        Ok(Response::new(SubModuleResp {
            success: true,
            resp_code: RespCode::Success.into(),
        }))
    }

    async fn heartbeat(
        &self,
        request: Request<SubmoduleReq>,
    ) -> std::result::Result<Response<SubModuleResp>, Status> {
        self.operate_module_sender
            .send(ModuleOperate::create_by_req(
                request.into_inner(),
                OperateType::HEARTBEAT,
            ))
            .unwrap();
        Ok(Response::new(SubModuleResp {
            success: true,
            resp_code: RespCode::Success.into(),
        }))
    }

    async fn update(
        &self,
        request: Request<SubmoduleReq>,
    ) -> std::result::Result<Response<SubModuleResp>, Status> {
        self.operate_module_sender
            .send(ModuleOperate::create_by_req(
                request.into_inner(),
                OperateType::UPDATE,
            ))
            .unwrap();
        Ok(Response::new(SubModuleResp {
            success: true,
            resp_code: RespCode::Success.into(),
        }))
    }
}

#[tonic::async_trait]
impl Instruct for InstructImpl {
    async fn send_instruct(
        &self,
        request: Request<InstructReq>,
    ) -> Result<Response<InstructResp>, Status> {
        self.instruct_sender
            .send(InstructEntity::create_by_req(request.into_inner()))
            .unwrap();
        Ok(Response::new(InstructResp {
            status: true,
            resp_code: RespCode::Success.into(),
        }))
    }
}

#[tonic::async_trait]
impl Manipulate for ManipulateImpl {
    async fn send_manipulate(
        &self,
        request: Request<ManipulateReq>,
    ) -> Result<Response<ManipulateResp>, Status> {
        self.manipulate_sender
            .send(ManipulateEntity::create_by_req(request.into_inner()))
            .unwrap();
        Ok(Response::new(ManipulateResp {
            status: true,
            resp_code: RespCode::Success.into(),
        }))
    }
}

impl SubmoduleImpl {
    pub fn init(operate_module_sender: UnboundedSender<ModuleOperate>) -> Self {
        SubmoduleImpl {
            operate_module_sender,
        }
    }
}

impl InstructImpl {
    pub fn init(sender: UnboundedSender<InstructEntity>) -> Self {
        InstructImpl {
            instruct_sender: sender,
        }
    }
}

impl ManipulateImpl {
    pub fn init(sender: UnboundedSender<ManipulateEntity>) -> Self {
        ManipulateImpl {
            manipulate_sender: sender,
        }
    }
}
