use async_trait::async_trait;
use nihility_common::instruct::instruct_client::InstructClient;
use nihility_common::instruct::instruct_server::Instruct;
use nihility_common::instruct::{InstructReq, InstructResp};
use nihility_common::manipulate::manipulate_client::ManipulateClient;
use nihility_common::manipulate::manipulate_server::Manipulate;
use nihility_common::manipulate::{ManipulateReq, ManipulateResp};
use nihility_common::response_code::RespCode;
use nihility_common::sub_module::sub_module_server::SubModule;
use nihility_common::sub_module::{ModuleInfo, SubModuleResp};
use tokio::sync::mpsc::Sender;
use tonic::transport::Channel;
use tonic::{Request, Response, Status};

use crate::communicat::{SendInstructOperate, SendManipulateOperate};
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::Module;
use crate::AppError;

#[async_trait]
impl SendInstructOperate for InstructClient<Channel> {
    async fn send(&mut self, instruct: InstructReq) -> Result<RespCode, AppError> {
        let result = self
            .send_instruct(Request::new(instruct))
            .await?
            .into_inner();
        if let Some(resp_code) = RespCode::from_i32(result.resp_code) {
            Ok(resp_code)
        } else {
            Err(AppError::GrpcModuleException("InstructClient".to_string()))
        }
    }
}

#[async_trait]
impl SendManipulateOperate for ManipulateClient<Channel> {
    async fn send(&mut self, manipulate: ManipulateReq) -> Result<RespCode, AppError> {
        let result = self
            .send_manipulate(Request::new(manipulate))
            .await?
            .into_inner();
        if let Some(resp_code) = RespCode::from_i32(result.resp_code) {
            Ok(resp_code)
        } else {
            Err(AppError::GrpcModuleException(
                "ManipulateClient".to_string(),
            ))
        }
    }
}

pub struct SubModuleImpl {
    module_sender: Sender<Module>,
}

pub struct InstructImpl {
    instruct_sender: Sender<InstructEntity>,
}

pub struct ManipulateImpl {
    manipulate_sender: Sender<ManipulateEntity>,
}

#[tonic::async_trait]
impl SubModule for SubModuleImpl {
    async fn register(
        &self,
        request: Request<ModuleInfo>,
    ) -> Result<Response<SubModuleResp>, Status> {
        let module = Module::create_by_req(request.into_inner()).await.unwrap();
        tracing::info!("start register model:{}", &module.name);
        self.module_sender.send(module).await.unwrap();
        Ok(Response::new(SubModuleResp {
            success: true,
            resp_code: RespCode::Success.into(),
        }))
    }

    async fn offline(
        &self,
        request: Request<ModuleInfo>,
    ) -> Result<Response<SubModuleResp>, Status> {
        todo!()
    }

    async fn keep_alive(
        &self,
        request: Request<ModuleInfo>,
    ) -> Result<Response<SubModuleResp>, Status> {
        todo!()
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

impl SubModuleImpl {
    pub fn init(sender: Sender<Module>) -> Self {
        SubModuleImpl {
            module_sender: sender,
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
