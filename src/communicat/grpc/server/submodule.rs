use nihility_common::response_code::{Resp, RespCode};
use nihility_common::submodule::submodule_server::Submodule;
use nihility_common::submodule::SubmoduleReq;
use tokio::sync::mpsc::UnboundedSender;
use tonic::{Request, Response, Status};

use crate::entity::submodule::{ModuleOperate, OperateType};

pub struct SubmoduleImpl {
    operate_module_sender: UnboundedSender<ModuleOperate>,
}

impl SubmoduleImpl {
    pub fn init(operate_module_sender: UnboundedSender<ModuleOperate>) -> Self {
        SubmoduleImpl {
            operate_module_sender,
        }
    }
}

#[tonic::async_trait]
impl Submodule for SubmoduleImpl {
    async fn register(&self, request: Request<SubmoduleReq>) -> Result<Response<Resp>, Status> {
        self.operate_module_sender
            .send(ModuleOperate::create_by_req(
                request.into_inner(),
                OperateType::Register,
            ))
            .unwrap();
        Ok(Response::new(Resp {
            code: RespCode::Success.into(),
        }))
    }

    async fn offline(&self, request: Request<SubmoduleReq>) -> Result<Response<Resp>, Status> {
        self.operate_module_sender
            .send(ModuleOperate::create_by_req(
                request.into_inner(),
                OperateType::Offline,
            ))
            .unwrap();
        Ok(Response::new(Resp {
            code: RespCode::Success.into(),
        }))
    }

    async fn heartbeat(&self, request: Request<SubmoduleReq>) -> Result<Response<Resp>, Status> {
        self.operate_module_sender
            .send(ModuleOperate::create_by_req(
                request.into_inner(),
                OperateType::Heartbeat,
            ))
            .unwrap();
        Ok(Response::new(Resp {
            code: RespCode::Success.into(),
        }))
    }

    async fn update(&self, request: Request<SubmoduleReq>) -> Result<Response<Resp>, Status> {
        self.operate_module_sender
            .send(ModuleOperate::create_by_req(
                request.into_inner(),
                OperateType::Update,
            ))
            .unwrap();
        Ok(Response::new(Resp {
            code: RespCode::Success.into(),
        }))
    }
}
