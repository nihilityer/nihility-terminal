extern crate nihility_common;

use tokio::sync::mpsc;

use crate::config::SummaryConfig;
use crate::core::encoder;
use crate::core::module_manager;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::ModuleOperate;
pub use crate::error::AppError;
use crate::log::Log;

mod communicat;
mod config;
mod core;
mod entity;
mod error;
mod log;

pub struct NihilityTerminal;

impl NihilityTerminal {
    pub async fn start() -> Result<(), AppError> {
        let summary_config: SummaryConfig = SummaryConfig::init()?;
        Log::init(&summary_config.log)?;

        let (module_operate_se, module_operate_re) =
            mpsc::channel::<ModuleOperate>(summary_config.core.module_manager.channel_buffer);
        let (instruct_se, instruct_re) =
            mpsc::channel::<InstructEntity>(summary_config.core.module_manager.channel_buffer);
        let (manipulate_se, manipulate_re) =
            mpsc::channel::<ManipulateEntity>(summary_config.core.module_manager.channel_buffer);

        let communicat_future = communicat::communicat_module_start(
            &summary_config.communicat,
            module_operate_se.clone(),
            instruct_se,
            manipulate_se,
        );

        let encoder = encoder::encoder_builder(&summary_config.core.encoder)?;
        let module_manager_future = module_manager::module_manager_builder(
            &summary_config.core.module_manager,
            encoder,
            module_operate_se.clone(),
            module_operate_re,
            instruct_re,
            manipulate_re,
        );

        tracing::info!("start run");

        tokio::try_join!(communicat_future, module_manager_future,)?;
        Ok(())
    }
}
