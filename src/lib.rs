extern crate nihility_common;

use color_eyre::Result;
use tokio::sync::mpsc;

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

        let (module_operate_se, module_operate_re) =
            mpsc::channel::<ModuleOperate>(summary_config.core.channel_buffer);
        let (instruct_se, instruct_re) =
            mpsc::channel::<InstructEntity>(summary_config.core.channel_buffer);
        let (manipulate_se, manipulate_re) =
            mpsc::channel::<ManipulateEntity>(summary_config.core.channel_buffer);

        let communicat_future = communicat::communicat_module_start(
            summary_config.communicat.clone(),
            module_operate_se.clone(),
            instruct_se,
            manipulate_se,
        );

        let core_future = core::core_start(
            summary_config.core.clone(),
            module_operate_se.clone(),
            module_operate_re,
            instruct_re,
            manipulate_re,
        );

        tokio::try_join!(communicat_future, core_future,)?;
        Ok(())
    }
}
