use anyhow::Result;
use async_trait::async_trait;
use nihility_common::instruct::InstructReq;
use nihility_common::manipulate::ManipulateReq;
use nihility_common::response_code::RespCode;
use tokio::sync::mpsc::Sender;
use tokio::try_join;

use crate::communicat::grpc::GrpcServer;
use crate::communicat::multicast::Multicast;
#[cfg(unix)]
use crate::communicat::pipe::PipeProcessor;
#[cfg(windows)]
use crate::communicat::windows_named_pipe::WindowsNamedPipeProcessor;
use crate::config::CommunicatConfig;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::ModuleOperate;

pub mod grpc;
mod multicast;
#[cfg(unix)]
pub mod pipe;
#[cfg(windows)]
pub mod windows_named_pipe;

/// 发送指令特征
#[async_trait]
pub trait SendInstructOperate {
    /// 发送指令
    async fn send(&mut self, instruct: InstructReq) -> Result<RespCode>;
}

/// 发送操作特征
#[async_trait]
pub trait SendManipulateOperate {
    /// 发送操作
    async fn send(&mut self, manipulate: ManipulateReq) -> Result<RespCode>;
}

pub async fn communicat_module_start(
    config: CommunicatConfig,
    operate_module_sender: Sender<ModuleOperate>,
    instruct_sender: Sender<InstructEntity>,
    manipulate_sender: Sender<ManipulateEntity>,
) -> Result<()> {
    let grpc_server_future = GrpcServer::start(
        &config.grpc,
        operate_module_sender.clone(),
        instruct_sender.clone(),
        manipulate_sender.clone(),
    );

    let multicast_future = Multicast::start(&config.multicast);

    #[cfg(unix)]
    let pipe_processor_future = PipeProcessor::start(
        &config.pipe,
        operate_module_sender.clone(),
        instruct_sender.clone(),
        manipulate_sender.clone(),
    );

    #[cfg(windows)]
    let pipe_processor_future = WindowsNamedPipeProcessor::start(
        &config.windows_named_pipes,
        operate_module_sender.clone(),
        instruct_sender.clone(),
        manipulate_sender.clone(),
    );

    try_join!(grpc_server_future, multicast_future, pipe_processor_future)?;
    Ok(())
}
