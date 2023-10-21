use async_trait::async_trait;

#[cfg(unix)]
use crate::communicat::pipe::{PipeUnixInstructClient, PipeUnixManipulateClient};

#[cfg(unix)]
#[async_trait]
impl SendInstructOperate for PipeUnixInstructClient {
    async fn send(
        &mut self,
        instruct: InstructReq
    ) -> Result<bool, AppError> {
        let result = self.send_instruct(instruct).await?;
        Ok(result.status)
    }
}

#[cfg(unix)]
#[async_trait]
impl SendManipulateOperate for PipeUnixManipulateClient {
    async fn send(
        &mut self,
        manipulate: ManipulateReq
    ) -> Result<bool, AppError> {
        let result = self.send_manipulate(manipulate).await?;
        Ok(result.status)
    }
}
