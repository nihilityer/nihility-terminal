use std::error::Error;

extern crate nihility_common;
mod exchange_actuator;

pub use exchange_actuator::{config::NetConfig, notify::{Broadcaster, GrpcServer}};
pub use nihility_common::submodule::info::SubmoduleInfo;
use tracing::Level;

pub struct SummaryConfig {
    pub net_cfg: NetConfig,
}

pub struct NihilityTerminal {
    submodule_vec: Vec<SubmoduleInfo>,
    grpc_server: GrpcServer,
    broadcaster: Broadcaster,
}

impl Default for SummaryConfig {
    fn default() -> Self {
        SummaryConfig {
            net_cfg: NetConfig::default().unwrap(),
        }
    }
}

impl NihilityTerminal {

    pub async fn init(summary_config: SummaryConfig) -> Result<Self, Box<dyn Error>> {
        let subscriber = tracing_subscriber::fmt()
            .compact()
            .with_max_level(Level::DEBUG)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_target(false)
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;

        tracing::info!("日志初始化完成！");

        let grpc_server = GrpcServer::init(&summary_config.net_cfg)?;

        let broadcaster = Broadcaster::init(&summary_config.net_cfg).await?;

        Ok(NihilityTerminal {
            submodule_vec: Vec::new(),
            grpc_server,
            broadcaster,
        })
    }

    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        tracing::info!("开始运行");

        let broadcaster_future = self.broadcaster.start();
        let grpc_server_future = self.grpc_server.start();

        tokio::try_join!(
            broadcaster_future,
            grpc_server_future,
        )?;

        Ok(())
    }

}
