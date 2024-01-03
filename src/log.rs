use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use time::macros::format_description;
use time::UtcOffset;
use tracing::debug;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::time::OffsetTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, Layer};

use crate::config::{LogConfig, LogOutType};

const LOG_FILE_NAME: &str = "nihility-terminal.log";

static CONSOLE_WORK_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
static FILE_WORK_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub struct Log;

impl Log {
    pub fn init(configs: &Vec<LogConfig>) -> Result<()> {
        let mut layers = Vec::new();

        for config in configs {
            if !config.enable {
                continue;
            }
            let non_blocking = match &config.out_type {
                LogOutType::Console => {
                    let (non_blocking, guard) = non_blocking(std::io::stdout());
                    CONSOLE_WORK_GUARD.get_or_init(|| guard);
                    non_blocking
                }
                LogOutType::File(out_path) => {
                    let file_appender = rolling::daily(out_path, LOG_FILE_NAME);
                    let (non_blocking, guard) = non_blocking(file_appender);
                    FILE_WORK_GUARD.get_or_init(|| guard);
                    non_blocking
                }
            };
            let timer = OffsetTime::new(
                UtcOffset::from_hms(8, 0, 0).unwrap(),
                format_description!(
                    "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
                ),
            );
            let layer = fmt::layer()
                .with_file(config.with_file)
                .with_line_number(config.with_line_number)
                .with_thread_ids(config.with_thread_ids)
                .with_target(config.with_target)
                .with_timer(timer)
                .with_writer(non_blocking);
            let layer = match config.level.to_lowercase().as_str() {
                "trace" => layer.with_filter(LevelFilter::TRACE),
                "debug" => layer.with_filter(LevelFilter::DEBUG),
                "info" => layer.with_filter(LevelFilter::INFO),
                "warn" => layer.with_filter(LevelFilter::WARN),
                "error" => layer.with_filter(LevelFilter::ERROR),
                other => {
                    return Err(anyhow!(
                        "Log Module Config Error : Level {other} Is Not Support"
                    ));
                }
            }
            .boxed();
            layers.push(layer);
        }
        tracing_subscriber::registry().with(layers).init();
        debug!("Log Subscriber Init Success");
        Ok(())
    }
}
