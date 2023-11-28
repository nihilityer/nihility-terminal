use anyhow::Result;
use async_trait::async_trait;
use nihility_common::instruct::InstructReq;
use nihility_common::manipulate::ManipulateReq;
use nihility_common::response_code::RespCode;
use tokio::spawn;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

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
pub mod mock;

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
    cancellation_token: CancellationToken,
    operate_module_sender: UnboundedSender<ModuleOperate>,
    instruct_sender: UnboundedSender<InstructEntity>,
    manipulate_sender: UnboundedSender<ManipulateEntity>,
) -> () {
    let (communicat_status_se, mut communicat_status_re) =
        tokio::sync::mpsc::unbounded_channel::<String>();

    let grpc_config = config.grpc.clone();
    let grpc_cancellation_token = cancellation_token.clone();
    let grpc_communicat_status_sender = communicat_status_se.clone();
    let grpc_operate_module_sender = operate_module_sender.clone();
    let grpc_instruct_sender = instruct_sender.clone();
    let grpc_manipulate_sender = manipulate_sender.clone();
    spawn(async move {
        if let Err(e) = GrpcServer::start(
            grpc_config,
            grpc_cancellation_token.clone(),
            grpc_operate_module_sender,
            grpc_instruct_sender,
            grpc_manipulate_sender,
        )
        .await
        {
            error!("Grpc Server Error: {}", e);
            grpc_cancellation_token.cancel();
        }
        grpc_communicat_status_sender
            .send("Grpc Server".to_string())
            .unwrap();
    });

    #[cfg(unix)]
    {
        let pipe_config = config.pipe.clone();
        let pipe_cancellation_token = cancellation_token.clone();
        let pipe_communicat_status_sender = communicat_status_se.clone();
        let pipe_operate_module_sender = operate_module_sender.clone();
        let pipe_instruct_sender = instruct_sender.clone();
        let pipe_manipulate_sender = manipulate_sender.clone();
        spawn(async move {
            if let Err(e) = PipeProcessor::start(
                pipe_config,
                pipe_cancellation_token.clone(),
                pipe_operate_module_sender,
                pipe_instruct_sender,
                pipe_manipulate_sender,
            )
            .await
            {
                error!("Pipe Processor Error: {}", e);
                pipe_cancellation_token.cancel();
            }
            pipe_communicat_status_sender
                .send("Pipe Processor".to_string())
                .unwrap();
        });
    }

    #[cfg(windows)]
    {
        let windows_named_pipes_config = config.windows_named_pipes.clone();
        let windows_named_pipes_cancellation_token = cancellation_token.clone();
        let windows_named_pipes_communicat_status_sender = communicat_status_se.clone();
        let windows_named_pipes_operate_module_sender = operate_module_sender.clone();
        let windows_named_pipes_instruct_sender = instruct_sender.clone();
        let windows_named_pipes_manipulate_sender = manipulate_sender.clone();
        spawn(async move {
            if let Err(e) = WindowsNamedPipeProcessor::start(
                windows_named_pipes_config,
                windows_named_pipes_cancellation_token.clone(),
                windows_named_pipes_operate_module_sender,
                windows_named_pipes_instruct_sender,
                windows_named_pipes_manipulate_sender,
            )
            .await
            {
                error!("Windows Named Pipe Processor Error: {}", e);
                windows_named_pipes_cancellation_token.cancel();
            }
            windows_named_pipes_communicat_status_sender
                .send("Windows Named Pipe Processor".to_string())
                .unwrap();
        });
    }
    drop(operate_module_sender);

    let multicast_config = config.multicast.clone();
    let multicast_cancellation_token = cancellation_token.clone();
    let multicast_communicat_status_sender = communicat_status_se.clone();
    spawn(async move {
        if let Err(e) = Multicast::start(multicast_config).await {
            error!("Multicast Error: {}", e);
            multicast_cancellation_token.cancel();
        }
        multicast_communicat_status_sender
            .send("Multicast".to_string())
            .unwrap();
    });

    spawn(async move {
        while let Some(communicat_name) = communicat_status_re.recv().await {
            debug!("{} Exit", communicat_name);
        }
        info!("All Communicat Module Exit");
        if !cancellation_token.is_cancelled() {
            cancellation_token.cancel();
        }
    });
}
