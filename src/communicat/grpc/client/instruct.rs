use anyhow::Result;
use async_trait::async_trait;
use nihility_common::instruct::instruct_client::InstructClient;
use nihility_common::instruct::TextInstruct;
use nihility_common::response_code::RespCode;
use tonic::transport::Channel;
use tonic::Request;

use crate::communicat::SendInstructOperate;

#[async_trait]
impl SendInstructOperate for InstructClient<Channel> {
    async fn send_text(&mut self, instruct: TextInstruct) -> Result<RespCode> {
        Ok(self
            .send_text_instruct(Request::new(instruct))
            .await?
            .into_inner()
            .code())
    }
}
