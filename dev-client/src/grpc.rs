use tonic::{ Request, Response, Status};

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

#[derive(Default)]
pub struct InstructImpl {}

#[derive(Default)]
pub struct ManipulateImpl {}

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