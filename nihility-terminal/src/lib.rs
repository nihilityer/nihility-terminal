extern crate nihility_common;

use nihility_common::instruct::InstructType;
use time::macros::format_description;
use time::UtcOffset;
use tokio::sync::mpsc;
use tracing::Level;

use crate::config::SummaryConfig;
use crate::core::grpc::GrpcServer;
use crate::core::module_manager::ModuleManager;
use crate::core::multicast::Multicast;
#[cfg(unix)]
use crate::core::pipe::PipeProcessor;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::Module;
pub use crate::error::AppError;

mod communicat;
mod config;
mod core;
mod entity;
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
                }
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
                format_description!(
                    "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
                ),
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

        let (grpc_module_se, module_re) =
            mpsc::channel::<Module>(summary_config.module_manager.channel_buffer);
        let (grpc_instruct_se, instruct_re) =
            mpsc::channel::<InstructEntity>(summary_config.module_manager.channel_buffer);
        let (grpc_manipulate_se, manipulate_re) =
            mpsc::channel::<ManipulateEntity>(summary_config.module_manager.channel_buffer);

        #[cfg(unix)]
        let pipe_module_se = grpc_module_se.clone();
        #[cfg(unix)]
        let pipe_instruct_se = grpc_instruct_se.clone();
        #[cfg(unix)]
        let pipe_manipulate_se = grpc_manipulate_se.clone();
        let test_instruct_se = grpc_instruct_se.clone();

        let multicaster_future = Multicast::start(&summary_config.multicast);
        let grpc_server_future = GrpcServer::start(
            &summary_config.grpc,
            grpc_module_se,
            grpc_instruct_se,
            grpc_manipulate_se,
        );
        #[cfg(unix)]
        let pipe_processor_future = PipeProcessor::start(
            &summary_config.pipe,
            pipe_module_se,
            pipe_instruct_se,
            pipe_manipulate_se,
        );
        let module_manager_future = ModuleManager::start(
            &summary_config,
            module_re,
            instruct_re,
            manipulate_re
        );

        tracing::info!("start run");
        test_instruct_se.send(
            InstructEntity {
                instruct_type: InstructType::DefaultType,
                message: vec!["test try".to_string()],
            }
        ).await;
        #[cfg(unix)]
        tokio::try_join!(
            multicaster_future,
            grpc_server_future,
            pipe_processor_future,
            module_manager_future,
        )?;
        #[cfg(windows)]
        tokio::try_join!(
            multicaster_future,
            grpc_server_future,
            module_manager_future,
        )?;
        Ok(())
    }
}
