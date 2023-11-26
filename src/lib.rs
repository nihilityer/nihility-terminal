extern crate nihility_common;

use anyhow::Result;
use tokio::sync::mpsc;
use tokio::{select, signal, spawn};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::config::SummaryConfig;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::ModuleOperate;
use crate::log::Log;

mod communicat;
mod config;
mod core;
mod entity;
mod log;

pub struct NihilityTerminal;

impl NihilityTerminal {
    pub async fn start() -> Result<()> {
        let summary_config: SummaryConfig = SummaryConfig::init()?;
        Log::init(&summary_config.log)?;

        let (module_operate_se, module_operate_re) = mpsc::unbounded_channel::<ModuleOperate>();
        let (shutdown_se, mut shutdown_re) = mpsc::unbounded_channel::<String>();
        let (instruct_se, instruct_re) = mpsc::unbounded_channel::<InstructEntity>();
        let (manipulate_se, manipulate_re) = mpsc::unbounded_channel::<ManipulateEntity>();

        let cancellation_token = CancellationToken::new();

        let communicat_config = summary_config.communicat.clone();
        let communicat_cancellation_token = cancellation_token.clone();
        let communicat_module_operate_se = module_operate_se.clone();
        spawn(async move {
            communicat::communicat_module_start(
                communicat_config,
                communicat_cancellation_token.clone(),
                communicat_module_operate_se,
                instruct_se,
                manipulate_se,
            )
            .await;
        });

        let core_config = summary_config.core.clone();
        let core_cancellation_token = cancellation_token.clone();
        let core_shutdown_se = shutdown_se.clone();
        let core_module_operate_se = module_operate_se.downgrade();
        drop(module_operate_se);
        spawn(async move {
            if let Err(e) = core::core_start(
                core_config,
                core_cancellation_token.clone(),
                core_shutdown_se,
                core_module_operate_se,
                module_operate_re,
                instruct_re,
                manipulate_re,
            )
            .await
            {
                error!("Core Error: {}", e);
                core_cancellation_token.cancel();
            }
        });

        select! {
            _ = deal_ctrl_c(cancellation_token.clone()) => {},
            _ = cancellation_token.cancelled() => {}
        }

        drop(shutdown_se);
        while let Some(module_name) = shutdown_re.recv().await {
            info!("{} Exit", module_name);
        }

        Ok(())
    }
}

async fn deal_ctrl_c(cancellation_token: CancellationToken) {
    match signal::ctrl_c().await {
        Ok(()) => {
            cancellation_token.cancel();
        }
        Err(_) => {}
    }
}
