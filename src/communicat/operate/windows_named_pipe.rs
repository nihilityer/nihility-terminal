use async_trait::async_trait;
use nihility_common::instruct::InstructReq;
use nihility_common::manipulate::ManipulateReq;
use crate::AppError;
use crate::communicat::operate::{SendInstructOperate, SendManipulateOperate};

#[cfg(windows)]
use crate::communicat::windows_named_pipe::{
    WindowsNamedPipeInstructClient, WindowsNamedPipeManipulateClient,
};

#[cfg(windows)]
#[async_trait]
impl SendInstructOperate for WindowsNamedPipeInstructClient {
    async fn send(&mut self, instruct: InstructReq) -> Result<bool, AppError> {
        let result = self.send_instruct(instruct).await?;
        Ok(result.status)
    }
}

#[cfg(windows)]
#[async_trait]
impl SendManipulateOperate for WindowsNamedPipeManipulateClient {
    async fn send(&mut self, manipulate: ManipulateReq) -> Result<bool, AppError> {
        let result = self.send_manipulate(manipulate).await?;
        Ok(result.status)
    }
}
