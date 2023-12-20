use std::sync::Mutex;

use anyhow::Result;
use nihility_common::{InstructEntity, ManipulateEntity, ModuleOperate};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, WeakUnboundedSender};
use tracing::info;

use crate::config::CoreConfig;
use crate::core::instruct_encoder::InstructEncoder;
use crate::core::instruct_matcher::InstructManager;
use crate::core::submodule_store::SubmoduleStore;

mod heartbeat_manager;
mod instruct_encoder;
mod instruct_manager;
mod instruct_matcher;
mod manipulate_manager;
mod submodule_manager;
mod submodule_store;

const HEARTBEAT_TIME: u64 = 30;

type AsyncMutex<T> = tokio::sync::Mutex<T>;

pub struct NihilityCore {
    pub instruct_encoder: Mutex<Box<dyn InstructEncoder + Send + Sync>>,
    pub instruct_matcher: AsyncMutex<Box<dyn InstructManager + Send + Sync>>,
    pub submodule_map: AsyncMutex<Box<dyn SubmoduleStore + Send + Sync>>,
}

pub struct NihilityCoreBuilder<MOF, IMF, MMF>
where
    MOF: Fn(UnboundedReceiver<ModuleOperate>) -> Result<()>,
    IMF: Fn(UnboundedReceiver<InstructEntity>) -> Result<()>,
    MMF: Fn(UnboundedReceiver<ManipulateEntity>) -> Result<()>,
{
    pub instruct_encoder: Option<Mutex<Box<dyn InstructEncoder + Send + Sync>>>,
    pub instruct_matcher: Option<AsyncMutex<Box<dyn InstructManager + Send + Sync>>>,
    pub submodule_store: Option<AsyncMutex<Box<dyn SubmoduleStore + Send + Sync>>>,
    pub module_operate_fn: MOF,
    pub instruct_manager_fn: IMF,
    pub manipulate_manager_fn: MMF,
}

impl<MOF, IMF, MMF> Default for NihilityCoreBuilder<MOF, IMF, MMF> {
    fn default() -> Self {
        NihilityCoreBuilder {
            instruct_encoder: None,
            instruct_matcher: None,
            submodule_store: None,
            module_operate_fn: |_| Ok(()),
            instruct_manager_fn: |_| Ok(()),
            manipulate_manager_fn: |_| Ok(()),
        }
    }
}
