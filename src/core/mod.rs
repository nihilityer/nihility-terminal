use std::sync::{Arc, OnceLock};

use anyhow::{anyhow, Result};
use nihility_common::{InstructEntity, ManipulateEntity, ModuleOperate};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;

use crate::core::instruct_encoder::InstructEncoder;
use crate::core::instruct_matcher::InstructMatcher;
use crate::core::operation_recorder::OperationRecorder;
use crate::core::submodule_store::SubmoduleStore;

pub mod core_thread;
pub mod instruct_encoder;
pub mod instruct_matcher;
pub mod operation_recorder;
pub mod submodule_store;

static CORE: OnceLock<NihilityCore> = OnceLock::new();

type InstructEncoderImpl = Arc<Box<dyn InstructEncoder + Send + Sync>>;
type InstructMatcherImpl = Arc<Mutex<Box<dyn InstructMatcher + Send + Sync>>>;
type SubmoduleStoreImpl = Arc<Mutex<Box<dyn SubmoduleStore + Send + Sync>>>;
type OperationRecorderImpl = Arc<Mutex<Box<dyn OperationRecorder + Send + Sync>>>;
type HeartbeatManagerFn = dyn Fn(SubmoduleStoreImpl) -> Result<()>;
type InstructManagerFn = dyn Fn(
    InstructEncoderImpl,
    InstructMatcherImpl,
    SubmoduleStoreImpl,
    OperationRecorderImpl,
    UnboundedReceiver<InstructEntity>,
) -> Result<()>;
type ManipulateManagerFn = dyn Fn(
    SubmoduleStoreImpl,
    OperationRecorderImpl,
    UnboundedReceiver<ManipulateEntity>,
) -> Result<()>;
type SubmoduleManagerFn = dyn Fn(
    InstructEncoderImpl,
    InstructMatcherImpl,
    SubmoduleStoreImpl,
    OperationRecorderImpl,
    UnboundedReceiver<ModuleOperate>,
) -> Result<()>;

pub struct NihilityCore {
    instruct_encoder: InstructEncoderImpl,
    instruct_matcher: InstructMatcherImpl,
    submodule_store: SubmoduleStoreImpl,
    operation_recorder: OperationRecorderImpl,
}

#[derive(Default)]
pub struct NihilityCoreBuilder {
    instruct_encoder: Option<Box<dyn InstructEncoder + Send + Sync>>,
    instruct_matcher: Option<Box<dyn InstructMatcher + Send + Sync>>,
    submodule_store: Option<Box<dyn SubmoduleStore + Send + Sync>>,
    operation_recorder: Option<Box<dyn OperationRecorder + Send + Sync>>,
    instruct_receiver: Option<UnboundedReceiver<InstructEntity>>,
    manipulate_receiver: Option<UnboundedReceiver<ManipulateEntity>>,
    module_operate_receiver: Option<UnboundedReceiver<ModuleOperate>>,
    heartbeat_manager_fn: Option<Box<HeartbeatManagerFn>>,
    instruct_manager_fn: Option<Box<InstructManagerFn>>,
    manipulate_manager_fn: Option<Box<ManipulateManagerFn>>,
    submodule_manager_fn: Option<Box<SubmoduleManagerFn>>,
}

impl NihilityCore {
    pub fn build(builder: NihilityCoreBuilder) -> Result<()> {
        match (
            builder.instruct_encoder,
            builder.instruct_matcher,
            builder.submodule_store,
            builder.operation_recorder,
            builder.instruct_receiver,
            builder.manipulate_receiver,
            builder.module_operate_receiver,
            builder.heartbeat_manager_fn,
            builder.instruct_manager_fn,
            builder.manipulate_manager_fn,
            builder.submodule_manager_fn,
        ) {
            (
                Some(instruct_encoder),
                Some(instruct_matcher),
                Some(submodule_store),
                Some(operation_recorder),
                Some(instruct_receiver),
                Some(manipulate_receiver),
                Some(module_operate_receiver),
                Some(heartbeat_manager_fn),
                Some(instruct_manager_fn),
                Some(manipulate_manager_fn),
                Some(submodule_manager_fn),
            ) => {
                let core = NihilityCore {
                    instruct_encoder: Arc::new(instruct_encoder),
                    instruct_matcher: Arc::new(Mutex::new(instruct_matcher)),
                    submodule_store: Arc::new(Mutex::new(submodule_store)),
                    operation_recorder: Arc::new(Mutex::new(operation_recorder)),
                };
                core.run_heartbeat_manager_fn(heartbeat_manager_fn)?;
                core.run_instruct_manager_fn(instruct_manager_fn, instruct_receiver)?;
                core.run_manipulate_manager_fn(manipulate_manager_fn, manipulate_receiver)?;
                core.run_submodule_manager_fn(submodule_manager_fn, module_operate_receiver)?;
                CORE.get_or_init(|| core);
                Ok(())
            }
            (None, _, _, _, _, _, _, _, _, _, _) => {
                Err(anyhow!("Builder instruct_encoder Field Value Is None"))
            }
            (_, None, _, _, _, _, _, _, _, _, _) => {
                Err(anyhow!("Builder instruct_matcher Field Value Is None"))
            }
            (_, _, None, _, _, _, _, _, _, _, _) => {
                Err(anyhow!("Builder submodule_store Field Value Is None"))
            }
            (_, _, _, None, _, _, _, _, _, _, _) => {
                Err(anyhow!("Builder operation_recorder Field Value Is None"))
            }
            (_, _, _, _, None, _, _, _, _, _, _) => {
                Err(anyhow!("Builder instruct_receiver Field Value Is None"))
            }
            (_, _, _, _, _, None, _, _, _, _, _) => {
                Err(anyhow!("Builder manipulate_receiver Field Value Is None"))
            }
            (_, _, _, _, _, _, None, _, _, _, _) => Err(anyhow!(
                "Builder module_operate_receiver Field Value Is None"
            )),
            (_, _, _, _, _, _, _, None, _, _, _) => {
                Err(anyhow!("Builder heartbeat_manager_fn Field Value Is None"))
            }
            (_, _, _, _, _, _, _, _, None, _, _) => {
                Err(anyhow!("Builder instruct_manager_fn Field Value Is None"))
            }
            (_, _, _, _, _, _, _, _, _, None, _) => {
                Err(anyhow!("Builder manipulate_manager_fn Field Value Is None"))
            }
            (_, _, _, _, _, _, _, _, _, _, None) => {
                Err(anyhow!("Builder submodule_manager_fn Field Value Is None"))
            }
        }
    }

    fn run_heartbeat_manager_fn(
        &self,
        heartbeat_manager_fn: Box<HeartbeatManagerFn>,
    ) -> Result<()> {
        heartbeat_manager_fn(self.submodule_store.clone())
    }

    fn run_instruct_manager_fn(
        &self,
        instruct_manager_fn: Box<InstructManagerFn>,
        receiver: UnboundedReceiver<InstructEntity>,
    ) -> Result<()> {
        instruct_manager_fn(
            self.instruct_encoder.clone(),
            self.instruct_matcher.clone(),
            self.submodule_store.clone(),
            self.operation_recorder.clone(),
            receiver,
        )
    }

    fn run_manipulate_manager_fn(
        &self,
        manipulate_manager_fn: Box<ManipulateManagerFn>,
        receiver: UnboundedReceiver<ManipulateEntity>,
    ) -> Result<()> {
        manipulate_manager_fn(
            self.submodule_store.clone(),
            self.operation_recorder.clone(),
            receiver,
        )
    }

    fn run_submodule_manager_fn(
        &self,
        submodule_manager_fn: Box<SubmoduleManagerFn>,
        receiver: UnboundedReceiver<ModuleOperate>,
    ) -> Result<()> {
        submodule_manager_fn(
            self.instruct_encoder.clone(),
            self.instruct_matcher.clone(),
            self.submodule_store.clone(),
            self.operation_recorder.clone(),
            receiver,
        )
    }
}

impl NihilityCoreBuilder {
    pub fn set_instruct_encoder(
        &mut self,
        instruct_encoder: Box<dyn InstructEncoder + Send + Sync>,
    ) {
        self.instruct_encoder = Some(instruct_encoder)
    }

    pub fn set_instruct_matcher(
        &mut self,
        instruct_matcher: Box<dyn InstructMatcher + Send + Sync>,
    ) {
        self.instruct_matcher = Some(instruct_matcher)
    }

    pub fn set_submodule_store(&mut self, submodule_store: Box<dyn SubmoduleStore + Send + Sync>) {
        self.submodule_store = Some(submodule_store)
    }

    pub fn set_operation_recorder(
        &mut self,
        operation_recorder: Box<dyn OperationRecorder + Send + Sync>,
    ) {
        self.operation_recorder = Some(operation_recorder)
    }

    pub fn set_instruct_receiver(&mut self, instruct_receiver: UnboundedReceiver<InstructEntity>) {
        self.instruct_receiver = Some(instruct_receiver)
    }

    pub fn set_manipulate_receiver(
        &mut self,
        manipulate_receiver: UnboundedReceiver<ManipulateEntity>,
    ) {
        self.manipulate_receiver = Some(manipulate_receiver)
    }

    pub fn set_module_operate_receiver(
        &mut self,
        module_operate_receiver: UnboundedReceiver<ModuleOperate>,
    ) {
        self.module_operate_receiver = Some(module_operate_receiver)
    }

    pub fn set_heartbeat_manager_fn(
        &mut self,
        heartbeat_manager_fn: impl Fn(SubmoduleStoreImpl) -> Result<()> + 'static,
    ) {
        self.heartbeat_manager_fn = Some(Box::new(heartbeat_manager_fn))
    }

    pub fn set_instruct_manager_fn(
        &mut self,
        instruct_manager_fn: impl Fn(
                InstructEncoderImpl,
                InstructMatcherImpl,
                SubmoduleStoreImpl,
                OperationRecorderImpl,
                UnboundedReceiver<InstructEntity>,
            ) -> Result<()>
            + 'static,
    ) {
        self.instruct_manager_fn = Some(Box::new(instruct_manager_fn))
    }

    pub fn set_manipulate_manager_fn(
        &mut self,
        manipulate_manager_fn: impl Fn(
                SubmoduleStoreImpl,
                OperationRecorderImpl,
                UnboundedReceiver<ManipulateEntity>,
            ) -> Result<()>
            + 'static,
    ) {
        self.manipulate_manager_fn = Some(Box::new(manipulate_manager_fn))
    }

    pub fn set_submodule_manager_fn(
        &mut self,
        submodule_manager_fn: impl Fn(
                InstructEncoderImpl,
                InstructMatcherImpl,
                SubmoduleStoreImpl,
                OperationRecorderImpl,
                UnboundedReceiver<ModuleOperate>,
            ) -> Result<()>
            + 'static,
    ) {
        self.submodule_manager_fn = Some(Box::new(submodule_manager_fn))
    }
}
