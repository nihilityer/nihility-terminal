use anyhow::Result;
use async_trait::async_trait;
use nihility_common::instruct::TextInstruct;
use nihility_common::manipulate::SimpleManipulate;
use nihility_common::response_code::RespCode;
use tokio::spawn;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, info};

#[cfg(unix)]
use crate::communicat::pipe::PipeProcessor;
#[cfg(windows)]
// use crate::communicat::windows_named_pipe::WindowsNamedPipeProcessor;
use crate::config::CommunicatConfig;
use crate::entity::instruct::TextInstructEntity;
use crate::entity::manipulate::SimpleManipulateEntity;
use crate::entity::submodule::ModuleOperate;
use crate::CANCELLATION_TOKEN;

pub mod grpc;
pub mod mock;
mod multicast;
#[cfg(unix)]
pub mod pipe;
#[cfg(windows)]
pub mod windows_named_pipe;

/// 发送指令特征
#[async_trait]
pub trait SendInstructOperate {
    /// 发送指令
    async fn send(&mut self, instruct: TextInstruct) -> Result<RespCode>;
}

/// 发送操作特征
#[async_trait]
pub trait SendManipulateOperate {
    /// 发送操作
    async fn send(&mut self, manipulate: SimpleManipulate) -> Result<RespCode>;
}

pub fn communicat_module_start(
    config: CommunicatConfig,
    operate_module_sender: UnboundedSender<ModuleOperate>,
    instruct_sender: UnboundedSender<TextInstructEntity>,
    manipulate_sender: UnboundedSender<SimpleManipulateEntity>,
) -> Result<()> {
    let (communicat_status_se, mut communicat_status_re) =
        tokio::sync::mpsc::unbounded_channel::<String>();

    grpc::start(
        config.grpc.clone(),
        communicat_status_se.clone(),
        operate_module_sender.clone(),
        instruct_sender.clone(),
        manipulate_sender.clone(),
    );

    #[cfg(unix)]
    {
        let pipe_config = config.pipe.clone();
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
                CANCELLATION_TOKEN.cancel();
            }
            pipe_communicat_status_sender
                .send("Pipe Processor".to_string())
                .unwrap();
        });
    }

    #[cfg(windows)]
    // WindowsNamedPipeProcessor::start_processor(
    //     config.windows_named_pipes.clone(),
    //     operate_module_sender.clone(),
    //     instruct_sender.clone(),
    //     manipulate_sender.clone(),
    // )?;

    multicast::start(config.multicast.clone(), communicat_status_se.clone());

    drop(operate_module_sender);
    spawn(async move {
        while let Some(communicat_name) = communicat_status_re.recv().await {
            debug!("{} Exit", communicat_name);
        }
        info!("All Communicat Module Exit");
        if !CANCELLATION_TOKEN.is_cancelled() {
            CANCELLATION_TOKEN.cancel();
        }
    });
    Ok(())
}
