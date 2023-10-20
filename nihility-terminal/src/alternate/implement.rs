use tokio::sync::mpsc::Sender;
use tonic::{Request, Response, Status};

use nihility_common::{
    instruct::{
        instruct_server::Instruct,
        InstructReq,
        InstructResp,
    },
    manipulate::{
        manipulate_server::Manipulate,
        ManipulateReq,
        ManipulateResp,
    },
    module_info::{
        module_info_server::ModuleInfo,
        ModuleInfoReq,
        ModuleInfoResp,
    }
};

use crate::alternate::module::{InstructEntity, ManipulateEntity, Module};

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
        let instruct = InstructEntity::create_by_req(request.into_inner());
        tracing::info!("get instruct:{:?}", instruct);
        self.instruct_sender.send(instruct).await.unwrap();
        Ok(Response::new(InstructResp {
            status: true,
        }))
    }
}

#[tonic::async_trait]
impl Manipulate for ManipulateImpl {
    async fn send_manipulate(&self, request: Request<ManipulateReq>) -> Result<Response<ManipulateResp>, Status> {
        let manipulate = ManipulateEntity::create_by_req(request.into_inner());
        tracing::info!("get manipulate:{:?}", manipulate);
        self.manipulate_sender.send(manipulate).await.unwrap();
        Ok(Response::new(ManipulateResp {
            status: true,
        }))
    }
}

impl ModuleInfoImpl {
    pub fn init(sender: Sender<Module>) -> Self{
        ModuleInfoImpl {
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
