use std::error::Error;
use tonic::{Request, Response, Status};
use tokio::sync::mpsc::Sender;

use nihility_common::module_info::{
    ModuleInfoReq,
    ModuleInfoResp,
    module_info_server::ModuleInfo,
};

use nihility_common::instruct::{
    InstructReq,
    InstructResp,
    instruct_server::Instruct,
};

use nihility_common::manipulate::{
    ManipulateReq,
    ManipulateResp,
    manipulate_server::Manipulate,
};

use crate::alternate::module::{
    Module,
    ManipulateEntity,
    InstructEntity,
};

pub struct ModuleInfoImpl {
    module_sender: Sender<Module>,
}

pub struct InstructImpl {
    instruct_sender: Sender<InstructEntity>,
}

pub struct ManipulateImpl {
    manipulate_sender: Sender<ManipulateEntity>,
}

#[tonic::async_trait]
impl ModuleInfo for ModuleInfoImpl {
    async fn register(&self, request: Request<ModuleInfoReq>) -> Result<Response<ModuleInfoResp>, Status> {
        let module = Module::create_by_req(request.into_inner()).await.unwrap();
        tracing::info!("start register module:{}", &module.name);
        self.module_sender.send(module).await.unwrap();
        Ok(Response::new(ModuleInfoResp {
            success: true,
        }))
    }
}

#[tonic::async_trait]
impl Instruct for InstructImpl {
    async fn send_instruct(&self, request: Request<InstructReq>) -> Result<Response<InstructResp>, Status> {
        let test_req = request.into_inner();
        tracing::info!("get info:{:?}", test_req);
        Ok(Response::new(InstructResp {
            status: true,
        }))
    }
}

#[tonic::async_trait]
impl Manipulate for ManipulateImpl {
    async fn send_manipulate(&self, request: Request<ManipulateReq>) -> Result<Response<ManipulateResp>, Status> {
        let test_req = request.into_inner();
        tracing::info!("get info:{:?}", test_req);
        Ok(Response::new(ManipulateResp {
            status: true,
        }))
    }
}

impl ModuleInfoImpl {
    pub fn init(sender: Sender<Module>) -> Result<Self, Box<dyn Error>> {
        Ok(ModuleInfoImpl {
            module_sender: sender,
        })
    }
}

impl InstructImpl {
    pub fn init(sender: Sender<InstructEntity>) -> Result<Self, Box<dyn Error>> {
        Ok(InstructImpl {
            instruct_sender: sender,
        })
    }
}

impl ManipulateImpl {
    pub fn init(sender: Sender<ManipulateEntity>) -> Result<Self, Box<dyn Error>> {
        Ok(ManipulateImpl {
            manipulate_sender: sender,
        })
    }
}
