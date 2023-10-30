use crate::AppError;
use nihility_common::instruct::{InstructReq, InstructResp};
use nihility_common::manipulate::{ManipulateReq, ManipulateResp};
use prost::Message;

#[cfg(unix)]
pub struct PipeUnixInstructClient {
    pub instruct_sender: Sender,
}

#[cfg(unix)]
pub struct PipeUnixManipulateClient {
    pub manipulate_sender: Sender,
}

#[cfg(unix)]
impl PipeUnixInstructClient {
    pub fn init(
        path: String
    ) -> Result<Self, AppError> {
        tracing::debug!("open instruct pipe sender from {}", &path);
        let sender = OpenOptions::new().open_sender(path)?;
        Ok(PipeUnixInstructClient {
            instruct_sender: sender,
        })
    }

    pub async fn send_instruct(
        &self,
        instruct_req: InstructReq
    ) -> Result<InstructResp, AppError> {
        loop {
            self.instruct_sender.writable().await?;
            let mut data = vec![0; 1024];
            instruct_req.encode(&mut data)?;

            match self.instruct_sender.try_write(&data) {
                Ok(_) => {
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        Ok(InstructResp { status: true })
    }
}

#[cfg(unix)]
impl PipeUnixManipulateClient {
    pub fn init(
        path: String
    ) -> Result<Self, AppError> {
        tracing::debug!("open manipulate pipe sender from {}", &path);
        let sender = OpenOptions::new().open_sender(path)?;
        Ok(PipeUnixManipulateClient {
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

            match self.manipulate_sender.try_write(&data) {
                Ok(_) => {
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        Ok(ManipulateResp { status: true })
    }
}
