use std::error::Error;
use tokio::sync::mpsc;

extern crate nihility_common;
mod alternate;

pub use alternate::{
    config::NetConfig,
    actuator::{Multicaster, GrpcServer, ModuleManager},
    module::{
        Module,
        InstructEntity,
        ManipulateEntity,
    },
};
use tracing::Level;

const CHANNEL_BUFFER:usize = 10;

pub struct SummaryConfig {
    pub net_cfg: NetConfig,
}

pub struct NihilityTerminal {
    module_manager: ModuleManager,
    grpc_server: GrpcServer,
    broadcaster: Multicaster,
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
        let (module_sender, module_receiver) = mpsc::channel::<Module>(CHANNEL_BUFFER);
        let (instruct_server_sender, instruct_server_receiver) = mpsc::channel::<InstructEntity>(CHANNEL_BUFFER);
        let (manipulate_server_sender, manipulate_server_receiver) = mpsc::channel::<ManipulateEntity>(CHANNEL_BUFFER);

        let broadcaster = Multicaster::init(&summary_config.net_cfg).await?;
        let grpc_server = GrpcServer::init(&summary_config.net_cfg, module_sender, instruct_server_sender, manipulate_server_sender)?;
        let module_manager = ModuleManager::init(module_receiver, instruct_server_receiver, manipulate_server_receiver, )?;


        Ok(NihilityTerminal {
            module_manager,
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
        );

        Ok(())
    }

}
