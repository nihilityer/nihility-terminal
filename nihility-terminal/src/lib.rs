use std::error::Error;
use tokio::sync::mpsc;
use tokio::fs;
use tokio::process::Command;
use regex::Regex;

extern crate nihility_common;
mod alternate;

pub use alternate::{
    config::NetConfig,
    actuator::{Multicaster, GrpcServer, FIFOProcessor, ModuleManager, FIFO_NAME},
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
    fifo_processor: FIFOProcessor,
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

        let exists = fs::try_exists(FIFO_NAME).await?;
        if !exists {
            let unix_re = Regex::new(r"^(/)?([^/]+(/)?)+/")?;
            let dir_path = unix_re.find(FIFO_NAME).unwrap();
            fs::create_dir_all(dir_path.as_str()).await?;
            Command::new("mkfifo").arg(FIFO_NAME).output().await.unwrap();
        }

        tracing::info!("日志初始化完成！");
        let (module_sender, module_receiver) = mpsc::channel::<Module>(CHANNEL_BUFFER);
        let (instruct_sender, instruct_receiver) = mpsc::channel::<InstructEntity>(CHANNEL_BUFFER);
        let (manipulate_sender, manipulate_receiver) = mpsc::channel::<ManipulateEntity>(CHANNEL_BUFFER);

        let broadcaster = Multicaster::init(&summary_config.net_cfg).await?;
        let grpc_server = GrpcServer::init(&summary_config.net_cfg, module_sender, instruct_sender, manipulate_sender)?;
        let fifo_processor = FIFOProcessor::init()?;
        let module_manager = ModuleManager::init(module_receiver, instruct_receiver, manipulate_receiver, )?;


        Ok(NihilityTerminal {
            module_manager,
            grpc_server,
            fifo_processor,
            broadcaster,
        })
    }

    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        tracing::info!("开始运行");

        let broadcaster_future = self.broadcaster.start();
        let grpc_server_future = self.grpc_server.start();
        let fifo_processor_future = self.fifo_processor.start();
        let module_manager_future = self.module_manager.start();

        let _ = tokio::try_join!(
            broadcaster_future,
            grpc_server_future,
            fifo_processor_future,
            module_manager_future,
        );

        Ok(())
    }

}
