use anyhow::Result;
use async_trait::async_trait;
use nihility_common::instruct::TextInstruct;
use nihility_common::manipulate::{SimpleManipulate, TextDisplayManipulate};
use nihility_common::response_code::RespCode;
use tracing::warn;

use crate::communicat::{SendInstructOperate, SendManipulateOperate};

#[derive(Default)]
pub struct MockInstructClient;

#[derive(Default)]
pub struct MockManipulateClient;

#[async_trait]
impl SendInstructOperate for MockInstructClient {
    async fn send_text(&mut self, instruct: TextInstruct) -> Result<RespCode> {
        warn!("Mock Instruct Client Get Instruct: {:?}", instruct);
        return Ok(RespCode::UnableToProcess);
    }
}

#[async_trait]
impl SendManipulateOperate for MockManipulateClient {
    async fn send_simple(&mut self, manipulate: SimpleManipulate) -> Result<RespCode> {
        warn!("Mock Manipulate Client Get Manipulate: {:?}", manipulate);
        return Ok(RespCode::UnableToProcess);
    }

    async fn send_text(&mut self, manipulate: TextDisplayManipulate) -> Result<RespCode> {
        warn!("Mock Manipulate Client Get Manipulate: {:?}", manipulate);
        return Ok(RespCode::UnableToProcess);
    }
}
