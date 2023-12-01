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
use tokio::spawn;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

use crate::communicat::{SendInstructOperate, SendManipulateOperate};
use crate::config::WindowsNamedPipesConfig;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::submodule::{ModuleOperate, OperateType};
use crate::CANCELLATION_TOKEN;

#[cfg(windows)]
pub struct WindowsNamedPipeProcessor;

#[cfg(windows)]
impl WindowsNamedPipeProcessor {
    pub fn start_processor(
        windows_named_pipe_config: WindowsNamedPipesConfig,
        module_operate_sender: UnboundedSender<ModuleOperate>,
        instruct_sender: UnboundedSender<InstructEntity>,
        manipulate_sender: UnboundedSender<ManipulateEntity>,
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

        let register_module_operate_sender = module_operate_sender.clone();
        spawn(async move {
            if let Err(e) =
                Self::register_named_pipe_processor(register_module_operate_sender, register_server)
                    .await
            {
                error!("register_named_pipe_processor Error: {}", e);
                CANCELLATION_TOKEN.cancel();
            }
            info!("register_named_pipe_processor Exit");
        });

        let offline_module_operate_sender = module_operate_sender.clone();
        spawn(async move {
            if let Err(e) =
                Self::offline_named_pipe_processor(offline_module_operate_sender, offline_server)
                    .await
            {
                error!("offline_named_pipe_processor Error: {}", e);
                CANCELLATION_TOKEN.cancel();
            }
            info!("offline_named_pipe_processor Exit");
        });

        let heartbeat_module_operate_sender = module_operate_sender.clone();
        spawn(async move {
            if let Err(e) = Self::heartbeat_named_pipe_processor(
                heartbeat_module_operate_sender,
                heartbeat_server,
            )
            .await
            {
                error!("heartbeat_named_pipe_processor Error: {}", e);
                CANCELLATION_TOKEN.cancel();
            }
            info!("heartbeat_named_pipe_processor Exit");
        });

        let update_module_operate_sender = module_operate_sender.clone();
        spawn(async move {
            if let Err(e) =
                Self::update_named_pipe_processor(update_module_operate_sender, update_server).await
            {
                error!("update_named_pipe_processor Error: {}", e);
                CANCELLATION_TOKEN.cancel();
            }
            info!("update_named_pipe_processor Exit");
        });

        spawn(async move {
            if let Err(e) =
                Self::instruct_named_pipe_processor(instruct_sender, instruct_server).await
            {
                error!("instruct_named_pipe_processor Error: {}", e);
                CANCELLATION_TOKEN.cancel();
            }
            info!("instruct_named_pipe_processor Exit");
        });

        spawn(async move {
            if let Err(e) =
                Self::manipulate_named_pipe_processor(manipulate_sender, manipulate_server).await
            {
                error!("manipulate_named_pipe_processor Error: {}", e);
                CANCELLATION_TOKEN.cancel();
            }
            info!("manipulate_named_pipe_processor Exit");
        });
        Ok(())
    }

    async fn register_named_pipe_processor(
        module_operate_sender: UnboundedSender<ModuleOperate>,
        register_server: NamedPipeServer,
    ) -> Result<()> {
        loop {
            if CANCELLATION_TOKEN.is_cancelled() {
                return Ok(());
            }
            register_server.readable().await?;
            let mut data = vec![0; 1024];

            match register_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(anyhow!("register_named_pipe_processor Read 0 Size"));
                }
                Ok(n) => {
                    let result: SubmoduleReq = SubmoduleReq::decode(&data[..n])?;
                    module_operate_sender
                        .send(ModuleOperate::create_by_req(result, OperateType::REGISTER))?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    async fn offline_named_pipe_processor(
        module_operate_sender: UnboundedSender<ModuleOperate>,
        offline_server: NamedPipeServer,
    ) -> Result<()> {
        loop {
            if CANCELLATION_TOKEN.is_cancelled() {
                return Ok(());
            }
            offline_server.readable().await?;
            let mut data = vec![0; 1024];

            match offline_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(anyhow!("offline_named_pipe_processor Read 0 Size"));
                }
                Ok(n) => {
                    let result: SubmoduleReq = SubmoduleReq::decode(&data[..n])?;
                    module_operate_sender
                        .send(ModuleOperate::create_by_req(result, OperateType::OFFLINE))?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    async fn heartbeat_named_pipe_processor(
        module_operate_sender: UnboundedSender<ModuleOperate>,
        heartbeat_server: NamedPipeServer,
    ) -> Result<()> {
        loop {
            if CANCELLATION_TOKEN.is_cancelled() {
                return Ok(());
            }
            heartbeat_server.readable().await?;
            let mut data = vec![0; 1024];

            match heartbeat_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(anyhow!("heartbeat_named_pipe_processor Read 0 Size"));
                }
                Ok(n) => {
                    let result: SubmoduleReq = SubmoduleReq::decode(&data[..n])?;
                    module_operate_sender
                        .send(ModuleOperate::create_by_req(result, OperateType::HEARTBEAT))?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    async fn update_named_pipe_processor(
        module_operate_sender: UnboundedSender<ModuleOperate>,
        update_server: NamedPipeServer,
    ) -> Result<()> {
        loop {
            if CANCELLATION_TOKEN.is_cancelled() {
                return Ok(());
            }
            update_server.readable().await?;
            let mut data = vec![0; 1024];

            match update_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(anyhow!("update_named_pipe_processor Read 0 Size"));
                }
                Ok(n) => {
                    let result: SubmoduleReq = SubmoduleReq::decode(&data[..n])?;
                    module_operate_sender
                        .send(ModuleOperate::create_by_req(result, OperateType::UPDATE))?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    async fn instruct_named_pipe_processor(
        instruct_sender: UnboundedSender<InstructEntity>,
        instruct_server: NamedPipeServer,
    ) -> Result<()> {
        loop {
            if CANCELLATION_TOKEN.is_cancelled() {
                return Ok(());
            }
            instruct_server.readable().await?;
            let mut data = vec![0; 1024];

            match instruct_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(anyhow!("instruct_named_pipe_processor Read 0 Size"));
                }
                Ok(n) => {
                    let result: InstructReq = InstructReq::decode(&data[..n])?;
                    instruct_sender.send(InstructEntity::create_by_req(result))?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    async fn manipulate_named_pipe_processor(
        manipulate_sender: UnboundedSender<ManipulateEntity>,
        manipulate_server: NamedPipeServer,
    ) -> Result<()> {
        loop {
            if CANCELLATION_TOKEN.is_cancelled() {
                return Ok(());
            }
            manipulate_server.readable().await?;
            let mut data = vec![0; 1024];

            match manipulate_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(anyhow!("manipulate_named_pipe_processor Read 0 Size"));
                }
                Ok(n) => {
                    let result: ManipulateReq = ManipulateReq::decode(&data[..n])?;
                    manipulate_sender.send(ManipulateEntity::create_by_req(result))?;
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
        debug!("Open Instruct Pipe UnboundedSender From {}", &path);
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

            match self.instruct_sender.try_write(&data) {
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
        debug!("Open Manipulate Pipe UnboundedSender From {}", &path);
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

            match self.manipulate_sender.try_write(&data) {
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
