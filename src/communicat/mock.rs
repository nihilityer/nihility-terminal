use anyhow::{anyhow, Result};
use async_trait::async_trait;
use nihility_common::instruct::TextInstruct;
use nihility_common::manipulate::{SimpleManipulate, TextDisplayManipulate};
use nihility_common::response_code::RespCode;
use tokio::sync::mpsc::Receiver;
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

    async fn send_multiple_text(&mut self, instruct_stream: Receiver<TextInstruct>) -> Result<Receiver<RespCode>> {
        warn!("Mock Instruct Client Get Instruct: {:?}", instruct_stream);
        return Err(anyhow!("Mock Instruct Cannot send_multiple_text"))
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
