use anyhow::Result;
use async_trait::async_trait;
use nihility_common::manipulate::manipulate_client::ManipulateClient;
use nihility_common::manipulate::{SimpleManipulate, TextDisplayManipulate};
use nihility_common::response_code::RespCode;
use tonic::transport::Channel;
use tonic::Request;

use crate::communicat::SendManipulateOperate;

#[async_trait]
impl SendManipulateOperate for ManipulateClient<Channel> {
    async fn send_simple(&mut self, manipulate: SimpleManipulate) -> Result<RespCode> {
        Ok(self
            .send_simple_manipulate(Request::new(manipulate))
            .await?
            .into_inner()
            .code())
    }

    async fn send_text(&mut self, manipulate: TextDisplayManipulate) -> Result<RespCode> {
        Ok(self
            .send_text_display_manipulate(Request::new(manipulate))
            .await?
            .into_inner()
            .code())
    }
}
