use anyhow::{anyhow, Result};
use time::macros::format_description;
use time::UtcOffset;
use tracing::Level;

use crate::config::LogConfig;

pub struct Log;

impl Log {
    pub fn init(config: &LogConfig) -> Result<()> {
        if !config.enable {
            return Ok(());
        }
        let mut subscriber = tracing_subscriber::fmt().compact();
        match config.level.to_lowercase().as_str() {
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
            other => {
                return Err(anyhow!(
                    "Log Module Config Error : Level {other} is not support"
                ));
            }
        }

        let timer = tracing_subscriber::fmt::time::OffsetTime::new(
            UtcOffset::from_hms(8, 0, 0).unwrap(),
            format_description!(
                "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
            ),
        );
        let subscriber = subscriber
            .with_file(config.with_file)
            .with_line_number(config.with_line_number)
            .with_thread_ids(config.with_thread_ids)
            .with_target(config.with_target)
            .with_timer(timer)
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;
        tracing::debug!("log subscriber init success");

        Ok(())
    }
}
