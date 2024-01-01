use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use time::macros::format_description;
use time::UtcOffset;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;

use crate::config::{LogConfig, LogOutType};

const LOG_FILE_NAME: &str = "nihility-terminal.log";

static WORK_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub struct Log;

impl Log {
    pub fn init(config: &LogConfig) -> Result<()> {
        if !config.enable {
            return Ok(());
        }
        let mut subscriber = match config.out_type {
            LogOutType::Console => {
                let subscriber = tracing_subscriber::fmt();
                let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
                WORK_GUARD.get_or_init(|| guard);
                subscriber.with_writer(non_blocking)
            }
            LogOutType::File => {
                let subscriber = tracing_subscriber::fmt();
                let file_appender =
                    tracing_appender::rolling::daily(config.out_path.as_str(), LOG_FILE_NAME);
                let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
                WORK_GUARD.get_or_init(|| guard);
                subscriber.with_writer(non_blocking)
            }
        };
        subscriber = match config.level.to_lowercase().as_str() {
            "trace" => subscriber.with_max_level(Level::TRACE),
            "debug" => subscriber.with_max_level(Level::DEBUG),
            "info" => subscriber.with_max_level(Level::INFO),
            "warn" => subscriber.with_max_level(Level::WARN),
            "error" => subscriber.with_max_level(Level::ERROR),
            other => {
                return Err(anyhow!(
                    "Log Module Config Error : Level {other} Is Not Support"
                ));
            }
        };

        let timer = tracing_subscriber::fmt::time::OffsetTime::new(
            UtcOffset::from_hms(8, 0, 0).unwrap(),
            format_description!(
                "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
            ),
        );
        let subscriber = subscriber
            .compact()
            .with_ansi(true)
            .with_file(config.with_file)
            .with_line_number(config.with_line_number)
            .with_thread_ids(config.with_thread_ids)
            .with_target(config.with_target)
            .with_timer(timer)
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;
        tracing::debug!("Log Subscriber Init Success");

        Ok(())
    }
}
