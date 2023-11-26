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
use tokio::try_join;
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
        info!("Windows Named Pipe Processor Start");
        let register_pipe_name = format!(
            r"{}\{}",
            &windows_named_pipe_config.pipe_prefix, &windows_named_pipe_config.register_pipe_name
        );
        let offline_pipe_name = format!(
            r"{}\{}",
            &windows_named_pipe_config.pipe_prefix, &windows_named_pipe_config.offline_pipe_name
        );
        let heartbeat_pipe_name = format!(
            r"{}\{}",
            &windows_named_pipe_config.pipe_prefix, &windows_named_pipe_config.heartbeat_pipe_name
        );
        let update_pipe_name = format!(
            r"{}\{}",
            &windows_named_pipe_config.pipe_prefix, &windows_named_pipe_config.update_pipe_name
        );
        let instruct_pipe_name = format!(
            r"{}\{}",
            &windows_named_pipe_config.pipe_prefix, &windows_named_pipe_config.instruct_pipe_name
        );
        let manipulate_pipe_name = format!(
            r"{}\{}",
            &windows_named_pipe_config.pipe_prefix, &windows_named_pipe_config.manipulate_pipe_name
        );

        let register_server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(register_pipe_name)?;
        let offline_server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(offline_pipe_name)?;
        let heartbeat_server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(heartbeat_pipe_name)?;
        let update_server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(update_pipe_name)?;
        let instruct_server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(instruct_pipe_name)?;
        let manipulate_server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(manipulate_pipe_name)?;

        try_join!(
            Self::register_named_pipe_processor(module_operate_sender.clone(), register_server),
            Self::offline_named_pipe_processor(module_operate_sender.clone(), offline_server),
            Self::heartbeat_named_pipe_processor(module_operate_sender.clone(), heartbeat_server),
            Self::update_named_pipe_processor(module_operate_sender.clone(), update_server),
            Self::instruct_named_pipe_processor(instruct_sender, instruct_server),
            Self::manipulate_named_pipe_processor(manipulate_sender, manipulate_server),
        )?;
        Ok(())
    }

    async fn register_named_pipe_processor(
        module_operate_sender: Sender<ModuleOperate>,
        module_server: NamedPipeServer,
    ) -> Result<()> {
        loop {
            module_server.readable().await?;
            let mut data = vec![0; 1024];

            match module_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(anyhow!("register_named_pipe_processor Read 0 Size"));
                }
                Ok(n) => {
                    let result: SubmoduleReq = SubmoduleReq::decode(&data[..n])?;
                    module_operate_sender
                        .send(ModuleOperate::create_by_req(result, OperateType::REGISTER))
                        .await?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    async fn offline_named_pipe_processor(
        module_operate_sender: Sender<ModuleOperate>,
        module_server: NamedPipeServer,
    ) -> Result<()> {
        loop {
            module_server.readable().await?;
            let mut data = vec![0; 1024];

            match module_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(anyhow!("offline_named_pipe_processor Read 0 Size"));
                }
                Ok(n) => {
                    let result: SubmoduleReq = SubmoduleReq::decode(&data[..n])?;
                    module_operate_sender
                        .send(ModuleOperate::create_by_req(result, OperateType::OFFLINE))
                        .await?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    async fn heartbeat_named_pipe_processor(
        module_operate_sender: Sender<ModuleOperate>,
        module_server: NamedPipeServer,
    ) -> Result<()> {
        loop {
            module_server.readable().await?;
            let mut data = vec![0; 1024];

            match module_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(anyhow!("heartbeat_named_pipe_processor Read 0 Size"));
                }
                Ok(n) => {
                    let result: SubmoduleReq = SubmoduleReq::decode(&data[..n])?;
                    module_operate_sender
                        .send(ModuleOperate::create_by_req(result, OperateType::HEARTBEAT))
                        .await?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    async fn update_named_pipe_processor(
        module_operate_sender: Sender<ModuleOperate>,
        module_server: NamedPipeServer,
    ) -> Result<()> {
        loop {
            module_server.readable().await?;
            let mut data = vec![0; 1024];

            match module_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(anyhow!("update_named_pipe_processor Read 0 Size"));
                }
                Ok(n) => {
                    let result: SubmoduleReq = SubmoduleReq::decode(&data[..n])?;
                    module_operate_sender
                        .send(ModuleOperate::create_by_req(result, OperateType::UPDATE))
                        .await?;
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
                    return Err(anyhow!("instruct_named_pipe_processor Read 0 Size"));
                }
                Ok(n) => {
                    let result: InstructReq = InstructReq::decode(&data[..n])?;
                    instruct_sender
                        .send(InstructEntity::create_by_req(result))
                        .await?;
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
                    return Err(anyhow!("manipulate_named_pipe_processor Read 0 Size"));
                }
                Ok(n) => {
                    let result: ManipulateReq = ManipulateReq::decode(&data[..n])?;
                    manipulate_sender
                        .send(ManipulateEntity::create_by_req(result))
                        .await?;
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
        debug!("Open Instruct Pipe Sender From {}", &path);
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
        debug!("Open Manipulate Pipe Sender From {}", &path);
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
