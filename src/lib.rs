extern crate nihility_common;

use time::macros::format_description;
use time::UtcOffset;
use tokio::sync::mpsc;
use tracing::Level;

use crate::config::SummaryConfig;
use crate::core::encoder;
use crate::core::grpc::GrpcServer;
use crate::core::module_manager;
use crate::core::multicast::Multicast;
#[cfg(unix)]
use crate::core::pipe::PipeProcessor;
use crate::core::windows_named_pipe::WindowsNamedPipeProcessor;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::Module;
pub use crate::error::AppError;

mod communicat;
mod config;
mod core;
mod entity;
mod error;

pub struct NihilityTerminal;

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

        let (module_se, module_re) =
            mpsc::channel::<Module>(summary_config.module_manager.channel_buffer);
        let (instruct_se, instruct_re) =
            mpsc::channel::<InstructEntity>(summary_config.module_manager.channel_buffer);
        let (manipulate_se, manipulate_re) =
            mpsc::channel::<ManipulateEntity>(summary_config.module_manager.channel_buffer);

        let multicast_future = Multicast::start(&summary_config.multicast);

        let grpc_server_future = GrpcServer::start(
            &summary_config.grpc,
            module_se.clone(),
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
            module_se.clone(),
            instruct_se.clone(),
            manipulate_se.clone(),
        );

        let encoder = encoder::encoder_builder(&summary_config.encoder)?;
        let module_manager_future = module_manager::module_manager_builder(
            &summary_config.module_manager,
            encoder,
            module_re,
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
