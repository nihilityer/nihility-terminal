use nihility_common::instruct::InstructReq;
use nihility_common::manipulate::ManipulateReq;
use nihility_common::module_info::ModuleInfoReq;
use prost::Message;
use tokio::io;
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::mpsc::Sender;

use crate::config::WindowsNamedPipesConfig;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::Module;
use crate::AppError;

#[cfg(windows)]
pub struct WindowsNamedPipeProcessor;

#[cfg(windows)]
impl WindowsNamedPipeProcessor {
    pub async fn start(
        windows_named_pipe_config: &WindowsNamedPipesConfig,
        module_sender: Sender<Module>,
        instruct_sender: Sender<InstructEntity>,
        manipulate_sender: Sender<ManipulateEntity>,
    ) -> Result<(), AppError> {
        if !windows_named_pipe_config.enable {
            return Ok(());
        }
        tracing::info!("WindowsNamedPipeProcessor start");
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
            Self::module_named_pipe_processor(module_sender, module_server),
            Self::instruct_named_pipe_processor(instruct_sender, instruct_server),
            Self::manipulate_named_pipe_processor(manipulate_sender, manipulate_server),
        )?;
        Ok(())
    }

    async fn module_named_pipe_processor(
        module_sender: Sender<Module>,
        module_server: NamedPipeServer,
    ) -> Result<(), AppError> {
        loop {
            module_server.readable().await?;
            let mut data = vec![0; 1024];

            match module_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(AppError::PipeError(
                        "module_named_pipe_processor read 0 size".to_string(),
                    ));
                }
                Ok(n) => {
                    let result: ModuleInfoReq = ModuleInfoReq::decode(&data[..n])?;
                    tracing::debug!("named pipe model name:{:?}", &result.name);
                    let model = Module::create_by_req(result).await?;
                    module_sender.send(model).await?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(_) => {
                    return Err(AppError::PipeError(
                        "module_named_pipe_processor".to_string(),
                    ));
                }
            }
        }
    }

    async fn instruct_named_pipe_processor(
        instruct_sender: Sender<InstructEntity>,
        instruct_server: NamedPipeServer,
    ) -> Result<(), AppError> {
        loop {
            instruct_server.readable().await?;
            let mut data = vec![0; 1024];

            match instruct_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(AppError::PipeError(
                        "instruct_named_pipe_processor read 0 size".to_string(),
                    ));
                }
                Ok(n) => {
                    let result: InstructReq = InstructReq::decode(&data[..n])?;
                    tracing::debug!("named pipe instruct name:{:?}", &result);
                    let instruct = InstructEntity::create_by_req(result);
                    instruct_sender.send(instruct).await?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(_) => {
                    return Err(AppError::PipeError(
                        "instruct_named_pipe_processor".to_string(),
                    ));
                }
            }
        }
    }

    async fn manipulate_named_pipe_processor(
        manipulate_sender: Sender<ManipulateEntity>,
        manipulate_server: NamedPipeServer,
    ) -> Result<(), AppError> {
        loop {
            manipulate_server.readable().await?;
            let mut data = vec![0; 1024];

            match manipulate_server.try_read(&mut data) {
                Ok(0) => {
                    return Err(AppError::PipeError(
                        "instruct_named_pipe_processor read 0 size".to_string(),
                    ));
                }
                Ok(n) => {
                    let result: ManipulateReq = ManipulateReq::decode(&data[..n])?;
                    tracing::debug!("named pipe manipulate name:{:?}", &result);
                    let manipulate = ManipulateEntity::create_by_req(result);
                    manipulate_sender.send(manipulate).await?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(_) => {
                    return Err(AppError::PipeError(
                        "manipulate_named_pipe_processor".to_string(),
                    ));
                }
            }
        }
    }
}
