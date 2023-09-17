extern crate nihility_common;

use tokio::sync::mpsc;
use tracing::Level;
use time::{macros::format_description, UtcOffset};

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
        if summary_config.log.enable {
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

            let timer = tracing_subscriber::fmt::time::OffsetTime::new(
                UtcOffset::from_hms(8, 0, 0).unwrap(),
                format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"),
            );
            let subscriber = subscriber
                .with_line_number(summary_config.log.with_line_number)
                .with_thread_ids(summary_config.log.with_thread_ids)
                .with_target(summary_config.log.with_target)
                .with_timer(timer)
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;
            tracing::info!("日志模块初始化完成！");
        }

        let (grpc_module_se, grpc_module_re) = mpsc::channel::<Module>(summary_config.module_manager.channel_buffer);
        let (grpc_instruct_se, grpc_instruct_re) = mpsc::channel::<InstructEntity>(summary_config.module_manager.channel_buffer);
        let (grpc_manipulate_se, grpc_manipulate_re) = mpsc::channel::<ManipulateEntity>(summary_config.module_manager.channel_buffer);

        let (pipe_module_se, pipe_module_re) = mpsc::channel::<Module>(summary_config.module_manager.channel_buffer);
        let (pipe_instruct_se, pipe_instruct_re) = mpsc::channel::<InstructEntity>(summary_config.module_manager.channel_buffer);
        let (pipe_manipulate_se, pipe_manipulate_re) = mpsc::channel::<ManipulateEntity>(summary_config.module_manager.channel_buffer);


        let multicaster_future = Multicaster::start(&summary_config.multicast);
        let grpc_server_future = GrpcServer::start(
            &summary_config.grpc,
            grpc_module_se,
            grpc_instruct_se,
            grpc_manipulate_se
        );
        let pipe_processor_future = PipeProcessor::start(
            &summary_config.pipe,
            pipe_module_se,
            pipe_instruct_se,
            pipe_manipulate_se
        );
        let module_manager_future = ModuleManager::start(
            &summary_config,
            grpc_module_re,
            grpc_instruct_re,
            grpc_manipulate_re,
            pipe_module_re,
            pipe_instruct_re,
            pipe_manipulate_re
        );

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
