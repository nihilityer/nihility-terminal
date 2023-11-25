use std::io;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use nihility_common::instruct::{InstructReq, InstructResp};
use nihility_common::manipulate::{ManipulateReq, ManipulateResp};
use nihility_common::response_code::RespCode;
use nihility_common::submodule::SubmoduleReq;
use prost::Message;
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::mpsc::Sender;
use tracing::{debug, info};

use crate::communicat::{SendInstructOperate, SendManipulateOperate};
use crate::config::WindowsNamedPipesConfig;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::{ModuleOperate, OperateType};

#[cfg(windows)]
pub struct WindowsNamedPipeProcessor;

#[cfg(windows)]
impl WindowsNamedPipeProcessor {
    pub async fn start(
        windows_named_pipe_config: &WindowsNamedPipesConfig,
        module_operate_sender: Sender<ModuleOperate>,
        instruct_sender: Sender<InstructEntity>,
        manipulate_sender: Sender<ManipulateEntity>,
    ) -> Result<()> {
        if !windows_named_pipe_config.enable {
            return Ok(());
        }
        info!("WindowsNamedPipeProcessor start");
        let module_pipe_name = format!(
            r"{}\{}",
            &windows_named_pipe_config.pipe_prefix, &windows_named_pipe_config.module_pipe_name
        );
        let instruct_pipe_name = format!(
            r"{}\{}",
            &windows_named_pipe_config.pipe_prefix, &windows_named_pipe_config.instruct_pipe_name
        );
        let manipulate_pipe_name = format!(
            r"{}\{}",
            &windows_named_pipe_config.pipe_prefix, &windows_named_pipe_config.manipulate_pipe_name
        );

        let module_server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(module_pipe_name)?;
        let instruct_server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(instruct_pipe_name)?;
        let manipulate_server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(manipulate_pipe_name)?;

        tokio::try_join!(
            Self::module_named_pipe_processor(module_operate_sender, module_server),
            Self::instruct_named_pipe_processor(instruct_sender, instruct_server),
            Self::manipulate_named_pipe_processor(manipulate_sender, manipulate_server),
        )?;
        Ok(())
    }

    async fn module_named_pipe_processor(
        module_operate_sender: Sender<ModuleOperate>,
        module_server: NamedPipeServer,
    ) -> Result<()> {
        loop {
            module_server.readable().await?;
            let mut data = vec![0; 1024];

            match module_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(anyhow!("module_named_pipe_processor read 0 size"));
                }
                Ok(n) => {
                    let result: SubmoduleReq = SubmoduleReq::decode(&data[..n])?;
                    debug!("named pipe model name:{:?}", &result.name);
                    let module_operate =
                        ModuleOperate::create_by_req(result, OperateType::REGISTER)?;
                    module_operate_sender.send(module_operate).await?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    async fn instruct_named_pipe_processor(
        instruct_sender: Sender<InstructEntity>,
        instruct_server: NamedPipeServer,
    ) -> Result<()> {
        loop {
            instruct_server.readable().await?;
            let mut data = vec![0; 1024];

            match instruct_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(anyhow!("instruct_named_pipe_processor read 0 size"));
                }
                Ok(n) => {
                    let result: InstructReq = InstructReq::decode(&data[..n])?;
                    debug!("named pipe instruct name:{:?}", &result);
                    let instruct = InstructEntity::create_by_req(result);
                    instruct_sender.send(instruct).await?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    async fn manipulate_named_pipe_processor(
        manipulate_sender: Sender<ManipulateEntity>,
        manipulate_server: NamedPipeServer,
    ) -> Result<()> {
        loop {
            manipulate_server.readable().await?;
            let mut data = vec![0; 1024];

            match manipulate_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(anyhow!("instruct_named_pipe_processor read 0 size"));
                }
                Ok(n) => {
                    let result: ManipulateReq = ManipulateReq::decode(&data[..n])?;
                    debug!("named pipe manipulate name:{:?}", &result);
                    let manipulate = ManipulateEntity::create_by_req(result);
                    manipulate_sender.send(manipulate).await?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }
}

#[cfg(windows)]
#[async_trait]
impl SendInstructOperate for WindowsNamedPipeInstructClient {
    async fn send(&mut self, instruct: InstructReq) -> Result<RespCode> {
        Ok(self.send_instruct(instruct).await?.resp_code())
    }
}

#[cfg(windows)]
#[async_trait]
impl SendManipulateOperate for WindowsNamedPipeManipulateClient {
    async fn send(&mut self, manipulate: ManipulateReq) -> Result<RespCode> {
        Ok(self.send_manipulate(manipulate).await?.resp_code())
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
    pub fn init(path: String) -> Result<Self> {
        debug!("open instruct pipe sender from {}", &path);
        let sender = ClientOptions::new().open(path)?;
        Ok(WindowsNamedPipeInstructClient {
            instruct_sender: sender,
        })
    }

    pub async fn send_instruct(&self, instruct_req: InstructReq) -> Result<InstructResp> {
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
    pub fn init(path: String) -> Result<Self> {
        debug!("open manipulate pipe sender from {}", &path);
        let sender = ClientOptions::new().open(path)?;
        Ok(WindowsNamedPipeManipulateClient {
            manipulate_sender: sender,
        })
    }

    pub async fn send_manipulate(&self, manipulate_req: ManipulateReq) -> Result<ManipulateResp> {
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
