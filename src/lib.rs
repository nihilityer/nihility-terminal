extern crate nihility_common;

use tokio::sync::mpsc;

use crate::config::SummaryConfig;
use crate::core::encoder;
use crate::core::grpc::GrpcServer;
use crate::core::module_manager;
use crate::core::multicast::Multicast;
#[cfg(unix)]
use crate::core::pipe::PipeProcessor;
#[cfg(windows)]
use crate::core::windows_named_pipe::WindowsNamedPipeProcessor;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::ModuleOperate;
pub use crate::error::AppError;
use crate::log::Log;

mod communicat;
mod config;
mod core;
mod entity;
mod error;
mod log;

pub struct NihilityTerminal;

impl NihilityTerminal {
    pub async fn start() -> Result<(), AppError> {
        let summary_config: SummaryConfig = SummaryConfig::init()?;
        Log::init(&summary_config.log)?;

        let (module_operate_se, module_operate_re) =
            mpsc::channel::<ModuleOperate>(summary_config.module_manager.channel_buffer);
        let (instruct_se, instruct_re) =
            mpsc::channel::<InstructEntity>(summary_config.module_manager.channel_buffer);
        let (manipulate_se, manipulate_re) =
            mpsc::channel::<ManipulateEntity>(summary_config.module_manager.channel_buffer);

        let multicast_future = Multicast::start(&summary_config.multicast);

        let grpc_server_future = GrpcServer::start(
            &summary_config.grpc,
            module_operate_se.clone(),
            instruct_se.clone(),
            manipulate_se.clone(),
        );

        #[cfg(unix)]
        let pipe_processor_future = PipeProcessor::start(
            &summary_config.pipe,
            module_se.clone(),
            instruct_se.clone(),
            manipulate_se.clone(),
        );

        #[cfg(windows)]
        let pipe_processor_future = WindowsNamedPipeProcessor::start(
            &summary_config.windows_named_pipes,
            module_operate_se.clone(),
            instruct_se.clone(),
            manipulate_se.clone(),
        );

        let encoder = encoder::encoder_builder(&summary_config.encoder)?;
        let module_manager_future = module_manager::module_manager_builder(
            &summary_config.module_manager,
            encoder,
            module_operate_re,
            instruct_re,
            manipulate_re,
        );

        tracing::info!("start run");

        #[cfg(unix)]
        tokio::try_join!(
            multicast_future,
            grpc_server_future,
            pipe_processor_future,
            module_manager_future,
        )?;

        #[cfg(windows)]
        tokio::try_join!(
            multicast_future,
            grpc_server_future,
            module_manager_future,
            pipe_processor_future,
        )?;
        Ok(())
    }
}
