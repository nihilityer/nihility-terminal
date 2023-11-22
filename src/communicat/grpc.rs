use async_trait::async_trait;
use color_eyre::{eyre::eyre, Result};
use nihility_common::instruct::instruct_client::InstructClient;
use nihility_common::instruct::instruct_server::{Instruct, InstructServer};
use nihility_common::instruct::{InstructReq, InstructResp};
use nihility_common::manipulate::manipulate_client::ManipulateClient;
use nihility_common::manipulate::manipulate_server::{Manipulate, ManipulateServer};
use nihility_common::manipulate::{ManipulateReq, ManipulateResp};
use nihility_common::response_code::RespCode;
use nihility_common::submodule::submodule_server::{Submodule, SubmoduleServer};
use nihility_common::submodule::{SubModuleResp, SubmoduleReq};
use tokio::sync::mpsc::Sender;
use tonic::transport::{Channel, Server};
use tonic::{Request, Response, Status};

use crate::communicat::{SendInstructOperate, SendManipulateOperate};
use crate::config::GrpcConfig;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::{ModuleOperate, OperateType};

pub struct GrpcServer;

impl GrpcServer {
    pub async fn start(
        grpc_config: &GrpcConfig,
        operate_module_sender: Sender<ModuleOperate>,
        instruct_sender: Sender<InstructEntity>,
        manipulate_sender: Sender<ManipulateEntity>,
    ) -> Result<()> {
        if grpc_config.enable {
            let bind_addr = format!(
                "{}:{}",
                grpc_config.addr.to_string(),
                grpc_config.port.to_string()
            );
            tracing::info!("Grpc Server bind at {}", &bind_addr);

            Server::builder()
                .add_service(SubmoduleServer::new(SubmoduleImpl::init(
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

#[async_trait]
impl SendInstructOperate for InstructClient<Channel> {
    async fn send(&mut self, instruct: InstructReq) -> Result<RespCode> {
        let result = self
            .send_instruct(Request::new(instruct))
            .await?
            .into_inner();
        if let Some(resp_code) = RespCode::from_i32(result.resp_code) {
            Ok(resp_code)
        } else {
            Err(eyre!(
                "Cannot transform RespCode from resp_code: {:?}",
                result.resp_code
            ))
        }
    }
}

#[async_trait]
impl SendManipulateOperate for ManipulateClient<Channel> {
    async fn send(&mut self, manipulate: ManipulateReq) -> Result<RespCode> {
        let result = self
            .send_manipulate(Request::new(manipulate))
            .await?
            .into_inner();
        if let Some(resp_code) = RespCode::from_i32(result.resp_code) {
            Ok(resp_code)
        } else {
            Err(eyre!(
                "Cannot transform RespCode from resp_code: {:?}",
                result.resp_code
            ))
        }
    }
}

pub struct SubmoduleImpl {
    operate_module_sender: Sender<ModuleOperate>,
}

pub struct InstructImpl {
    instruct_sender: Sender<InstructEntity>,
}

pub struct ManipulateImpl {
    manipulate_sender: Sender<ManipulateEntity>,
}

#[tonic::async_trait]
impl Submodule for SubmoduleImpl {
    async fn register(
        &self,
        request: Request<SubmoduleReq>,
    ) -> Result<Response<SubModuleResp>, Status> {
        let module =
            ModuleOperate::create_by_req(request.into_inner(), OperateType::REGISTER).unwrap();
        tracing::info!("start register model:{}", &module.name);
        self.operate_module_sender.send(module).await.unwrap();
        Ok(Response::new(SubModuleResp {
            success: true,
            resp_code: RespCode::Success.into(),
        }))
    }

    async fn offline(
        &self,
        request: Request<SubmoduleReq>,
    ) -> Result<Response<SubModuleResp>, Status> {
        let module =
            ModuleOperate::create_by_req(request.into_inner(), OperateType::OFFLINE).unwrap();
        tracing::info!("start offline model:{}", &module.name);
        self.operate_module_sender.send(module).await.unwrap();
        Ok(Response::new(SubModuleResp {
            success: true,
            resp_code: RespCode::Success.into(),
        }))
    }

    async fn keep_alive(
        &self,
        request: Request<SubmoduleReq>,
    ) -> Result<Response<SubModuleResp>, Status> {
        let module =
            ModuleOperate::create_by_req(request.into_inner(), OperateType::HEARTBEAT).unwrap();
        tracing::info!("get model:{} heartbeat", &module.name);
        self.operate_module_sender.send(module).await.unwrap();
        Ok(Response::new(SubModuleResp {
            success: true,
            resp_code: RespCode::Success.into(),
        }))
    }

    async fn update(
        &self,
        request: Request<SubmoduleReq>,
    ) -> std::result::Result<Response<SubModuleResp>, Status> {
        let module =
            ModuleOperate::create_by_req(request.into_inner(), OperateType::UPDATE).unwrap();
        tracing::info!("get model:{} update info", &module.name);
        self.operate_module_sender.send(module).await.unwrap();
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
        let instruct = InstructEntity::create_by_req(request.into_inner());
        tracing::info!("get instruct:{:?}", instruct);
        self.instruct_sender.send(instruct).await.unwrap();
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
        let manipulate = ManipulateEntity::create_by_req(request.into_inner());
        tracing::info!("get manipulate:{:?}", manipulate);
        self.manipulate_sender.send(manipulate).await.unwrap();
        Ok(Response::new(ManipulateResp {
            status: true,
            resp_code: RespCode::Success.into(),
        }))
    }
}

impl SubmoduleImpl {
    pub fn init(operate_module_sender: Sender<ModuleOperate>) -> Self {
        SubmoduleImpl {
            operate_module_sender,
        }
    }
}

impl InstructImpl {
    pub fn init(sender: Sender<InstructEntity>) -> Self {
        InstructImpl {
            instruct_sender: sender,
        }
    }
}

impl ManipulateImpl {
    pub fn init(sender: Sender<ManipulateEntity>) -> Self {
        ManipulateImpl {
            manipulate_sender: sender,
        }
    }
}
