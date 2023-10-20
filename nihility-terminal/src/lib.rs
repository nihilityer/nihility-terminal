extern crate nihility_common;

use time::{macros::format_description, UtcOffset};
use tokio::sync::mpsc;
use tracing::Level;

use alternate::{
    actuator::{GrpcServer, Multicaster, PipeProcessor},
    module_manager::ModuleManager,
    config::SummaryConfig,
    module::{
        InstructEntity,
        ManipulateEntity,
        Module,
    },
};
pub use error::AppError;
use nihility_common::instruct::InstructType;

mod alternate;
mod error;

pub struct NihilityTerminal {}

impl NihilityTerminal {
    pub async fn start() -> Result<(), AppError> {
        let summary_config: SummaryConfig = SummaryConfig::init()?;
        if summary_config.log.enable {
            let mut subscriber = tracing_subscriber::fmt().compact();
            match summary_config.log.level.to_lowercase().as_str() {
                "trace" => {
                    subscriber = subscriber.with_max_level(Level::TRACE);
                },
                "debug" => {
                    subscriber = subscriber.with_max_level(Level::DEBUG);
                }
                "info" => {
                    subscriber = subscriber.with_max_level(Level::INFO);
                }
                "warn" => {
                    subscriber = subscriber.with_max_level(Level::WARN);
                }
                "error" => {
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
                .with_file(summary_config.log.with_file)
                .with_line_number(summary_config.log.with_line_number)
                .with_thread_ids(summary_config.log.with_thread_ids)
                .with_target(summary_config.log.with_target)
                .with_timer(timer)
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;
            tracing::debug!("log subscriber init success");
        }

        let (pipe_module_se, module_re) = mpsc::channel::<Module>(summary_config.module_manager.channel_buffer);
        let (pipe_instruct_se, instruct_re) = mpsc::channel::<InstructEntity>(summary_config.module_manager.channel_buffer);
        let (pipe_manipulate_se, manipulate_re) = mpsc::channel::<ManipulateEntity>(summary_config.module_manager.channel_buffer);

        let grpc_module_se = pipe_module_se.clone();
        let grpc_instruct_se = pipe_instruct_se.clone();
        let test_instruct_se = pipe_instruct_se.clone();
        let grpc_manipulate_se = pipe_manipulate_se.clone();


        let multicaster_future = Multicaster::start(&summary_config.multicast);
        let grpc_server_future = GrpcServer::start(
            &summary_config.grpc,
            pipe_module_se,
            grpc_instruct_se,
            grpc_manipulate_se,
        );
        let pipe_processor_future = PipeProcessor::start(
            &summary_config.pipe,
            grpc_module_se,
            pipe_instruct_se,
            pipe_manipulate_se,
        );
        let module_manager_future = ModuleManager::start(
            &summary_config,
            module_re,
            instruct_re,
            manipulate_re,
        );

        tracing::info!("start run");
        test_instruct_se.send(InstructEntity {
            instruct_type: InstructType::DefaultType,
            message: vec!["test try".to_string()]
        }).await;
        tokio::try_join!(
            multicaster_future,
            grpc_server_future,
            pipe_processor_future,
            module_manager_future,
        )?;
        Ok(())
    }
}
