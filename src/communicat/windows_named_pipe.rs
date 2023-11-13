use std::io;

use async_trait::async_trait;
use nihility_common::instruct::{InstructReq, InstructResp};
use nihility_common::manipulate::{ManipulateReq, ManipulateResp};
use nihility_common::response_code::RespCode;
use prost::Message;
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};

use crate::communicat::{SendInstructOperate, SendManipulateOperate};
use crate::AppError;

#[cfg(windows)]
#[async_trait]
impl SendInstructOperate for WindowsNamedPipeInstructClient {
    async fn send(&mut self, instruct: InstructReq) -> Result<RespCode, AppError> {
        let result = self.send_instruct(instruct).await?;
        if let Some(resp_code) = RespCode::from_i32(result.resp_code) {
            Ok(resp_code)
        } else {
            Err(AppError::GrpcModuleException(
                "WindowsNamedPipeInstructClient".to_string(),
            ))
        }
    }
}

#[cfg(windows)]
#[async_trait]
impl SendManipulateOperate for WindowsNamedPipeManipulateClient {
    async fn send(&mut self, manipulate: ManipulateReq) -> Result<RespCode, AppError> {
        let result = self.send_manipulate(manipulate).await?;
        if let Some(resp_code) = RespCode::from_i32(result.resp_code) {
            Ok(resp_code)
        } else {
            Err(AppError::GrpcModuleException(
                "WindowsNamedPipeManipulateClient".to_string(),
            ))
        }
    }
}

#[cfg(windows)]
pub struct WindowsNamedPipeInstructClient {
    pub instruct_sender: NamedPipeClient,
}

#[cfg(windows)]
pub struct WindowsNamedPipeManipulateClient {
    pub manipulate_sender: NamedPipeClient,
}

#[cfg(windows)]
impl WindowsNamedPipeInstructClient {
    pub fn init(path: String) -> Result<Self, AppError> {
        tracing::debug!("open instruct pipe sender from {}", &path);
        let sender = ClientOptions::new().open(path)?;
        Ok(WindowsNamedPipeInstructClient {
            instruct_sender: sender,
        })
    }

    pub async fn send_instruct(&self, instruct_req: InstructReq) -> Result<InstructResp, AppError> {
        loop {
            self.instruct_sender.writable().await?;
            let mut data = vec![0; 1024];
            instruct_req.encode(&mut data)?;

            match self.instruct_sender.try_write(&*data) {
                Ok(_) => {
                    break;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        Ok(InstructResp {
            status: true,
            resp_code: RespCode::Success.into(),
        })
    }
}

#[cfg(windows)]
impl WindowsNamedPipeManipulateClient {
    pub fn init(path: String) -> Result<Self, AppError> {
        tracing::debug!("open manipulate pipe sender from {}", &path);
        let sender = ClientOptions::new().open(path)?;
        Ok(WindowsNamedPipeManipulateClient {
            manipulate_sender: sender,
        })
    }

    pub async fn send_manipulate(
        &self,
        manipulate_req: ManipulateReq,
    ) -> Result<ManipulateResp, AppError> {
        loop {
            self.manipulate_sender.writable().await?;
            let mut data = vec![0; 1024];
            manipulate_req.encode(&mut data)?;

            match self.manipulate_sender.try_write(&*data) {
                Ok(_) => {
                    break;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        Ok(ManipulateResp {
            status: true,
            resp_code: RespCode::Success.into(),
        })
    }
}
