extern crate nihility_common;

use tokio::sync::mpsc;
use tracing::Level;

pub use alternate::{
    actuator::{GrpcServer, ModuleManager, Multicaster, PipeProcessor},
    config::SummaryConfig,
    module::{
        InstructEntity,
        ManipulateEntity,
        Module,
    },
};
pub use error::AppError;

mod alternate;
mod error;

pub struct NihilityTerminal {}

impl NihilityTerminal {
    pub async fn start() -> Result<(), AppError> {
        let summary_config: SummaryConfig = SummaryConfig::init()?; // TODO
        let mut subscriber = tracing_subscriber::fmt().compact();

        match summary_config.log.level.as_str() {
            "debug" | "DEBUG" => {
                subscriber = subscriber.with_max_level(Level::DEBUG);
            }
            "info" | "INFO" => {
                subscriber = subscriber.with_max_level(Level::INFO);
            }
            "warn" | "WARN" => {
                subscriber = subscriber.with_max_level(Level::WARN);
            }
            "error" | "ERROR" => {
                subscriber = subscriber.with_max_level(Level::ERROR);
            }
            _ => {
                return Err(AppError::ConfigError("log".to_string()));
            }
        }

        let subscriber = subscriber
            .with_line_number(summary_config.log.with_line_number)
            .with_thread_ids(summary_config.log.with_thread_ids)
            .with_target(summary_config.log.with_target)
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;
        tracing::info!("日志初始化完成！");

        let (module_sender, module_receiver) = mpsc::channel::<Module>(summary_config.module_manager.channel_buffer);
        let (instruct_sender, instruct_receiver) = mpsc::channel::<InstructEntity>(summary_config.module_manager.channel_buffer);
        let (manipulate_sender, manipulate_receiver) = mpsc::channel::<ManipulateEntity>(summary_config.module_manager.channel_buffer);


        let multicaster_future = Multicaster::start(&summary_config.multicast);
        let grpc_server_future = GrpcServer::start(&summary_config.grpc, module_sender, instruct_sender, manipulate_sender);
        let pipe_processor_future = PipeProcessor::start(&summary_config.pipe);
        let module_manager_future = ModuleManager::start(&summary_config.module_manager, module_receiver, instruct_receiver, manipulate_receiver);

        tracing::info!("开始运行");
        tokio::try_join!(
            multicaster_future,
            grpc_server_future,
            pipe_processor_future,
            module_manager_future,
        )?;
        Ok(())
    }
}
