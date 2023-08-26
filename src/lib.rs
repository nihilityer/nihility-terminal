use std::error::Error;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;

mod exchange_actuator;
mod submodule;

pub use exchange_actuator::{config::NetConfig, notify::Broadcaster};
pub use submodule::info::SubmoduleInfo;

pub struct SummaryConfig {
    pub net_cfg: NetConfig,
}

pub struct NihilityTerminal {
    submodule_vec: Vec<SubmoduleInfo>,
    broadcaster: Broadcaster,
    broadcaster_rc: Sender<String>,
}

impl Default for SummaryConfig {
    fn default() -> Self {
        SummaryConfig {
            net_cfg: NetConfig::default(),
        }
    }
}

impl NihilityTerminal {

    pub async fn init(summary_config: SummaryConfig) -> Result<Self, Box<dyn Error>> {
        let subscriber = tracing_subscriber::fmt()
            .compact()
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_target(false)
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;

        tracing::info!("日志初始化完成！");

        let (rc, mut rx) = oneshot::channel();

        let broadcaster = Broadcaster::new(summary_config.net_cfg, rx).await?;

        Ok(NihilityTerminal {
            submodule_vec: Vec::new(),
            broadcaster,
            broadcaster_rc: rc
        })
    }

    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        tracing::info!("开始运行");

        let broadcaster_future = self.broadcaster.start();
        self.broadcaster_rc.send("test".to_string())?;

        tokio::join!(broadcaster_future);

        Ok(())
    }

}
