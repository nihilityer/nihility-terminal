use async_trait::async_trait;
use nihility_common::instruct::instruct_client::InstructClient;
use nihility_common::instruct::InstructReq;
use nihility_common::manipulate::manipulate_client::ManipulateClient;
use nihility_common::manipulate::ManipulateReq;
use tonic::Request;
use tonic::transport::Channel;

use crate::AppError;
use crate::communicat::operate::{SendInstructOperate, SendManipulateOperate};

#[async_trait]
impl SendInstructOperate for InstructClient<Channel> {
    async fn send(
        &mut self,
        instruct: InstructReq
    ) -> Result<bool, AppError> {
        let result = self
            .send_instruct(Request::new(instruct))
            .await?
            .into_inner();
        Ok(result.status)
    }
}

#[async_trait]
impl SendManipulateOperate for ManipulateClient<Channel> {
    async fn send(
        &mut self,
        manipulate: ManipulateReq
    ) -> Result<bool, AppError> {
        let result = self
            .send_manipulate(Request::new(manipulate))
            .await?
            .into_inner();
        Ok(result.status)
    }
}
