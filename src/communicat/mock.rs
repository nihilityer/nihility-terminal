use async_trait::async_trait;
use nihility_common::instruct::InstructReq;
use nihility_common::response_code::RespCode;
use tracing::warn;
use anyhow::Result;
use nihility_common::manipulate::ManipulateReq;
use crate::communicat::{SendInstructOperate, SendManipulateOperate};

#[derive(Default)]
pub struct MockInstructClient;

#[derive(Default)]
pub struct MockManipulateClient;

#[async_trait]
impl SendInstructOperate for MockInstructClient {
    async fn send(&mut self, instruct: InstructReq) -> Result<RespCode> {
        warn!("Mock Instruct Client Get Instruct: {:?}", instruct);
        return Ok(RespCode::UnableToProcess)
    }
}

#[async_trait]
impl SendManipulateOperate for MockManipulateClient {
    async fn send(&mut self, manipulate: ManipulateReq) -> Result<RespCode> {
        warn!("Mock Manipulate Client Get Manipulate: {:?}", manipulate);
        return Ok(RespCode::UnableToProcess)
    }
}