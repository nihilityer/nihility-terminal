extern crate nihility_common;

use anyhow::Result;
use lazy_static::lazy_static;
use tokio::sync::mpsc;
use tokio::{select, signal};
use tokio_util::sync::CancellationToken;
use tracing::info;

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

lazy_static! {
    pub(crate) static ref CANCELLATION_TOKEN: CancellationToken = CancellationToken::new();
}

pub struct NihilityTerminal;

impl NihilityTerminal {
    pub async fn start() -> Result<()> {
        let summary_config: SummaryConfig = SummaryConfig::init()?;

        Log::init(&summary_config.log)?;

        let (module_operate_se, module_operate_re) = mpsc::unbounded_channel::<ModuleOperate>();
        let (shutdown_se, mut shutdown_re) = mpsc::unbounded_channel::<String>();
        let (instruct_se, instruct_re) = mpsc::unbounded_channel::<InstructEntity>();
        let (manipulate_se, manipulate_re) = mpsc::unbounded_channel::<ManipulateEntity>();

        communicat::communicat_module_start(
            summary_config.communicat.clone(),
            module_operate_se.clone(),
            instruct_se,
            manipulate_se,
        )?;

        core::core_start(
            summary_config.core,
            shutdown_se.clone(),
            module_operate_se.downgrade(),
            module_operate_re,
            instruct_re,
            manipulate_re,
        )
        .await?;

        drop(shutdown_se);
        drop(module_operate_se);
        select! {
            _ = signal::ctrl_c() => {
                CANCELLATION_TOKEN.cancel();
            },
            _ = CANCELLATION_TOKEN.cancelled() => {}
        }
        while let Some(module_name) = shutdown_re.recv().await {
            info!("{} Exit", module_name);
        }

        Ok(())
    }
}
