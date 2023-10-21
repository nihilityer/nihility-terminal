use std::sync::{Arc, Mutex};

use tokio::sync::mpsc::Receiver;

use crate::AppError;
use crate::config::SummaryConfig;
use crate::core::encoder::InstructEncoder;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::Module;

pub struct ModuleManager;

impl ModuleManager {
    pub async fn start(
        summary_config: &SummaryConfig,
        module_receiver: Receiver<Module>,
        instruct_receiver: Receiver<InstructEntity>,
        manipulate_receiver: Receiver<ManipulateEntity>,
    ) -> Result<(), AppError> {
        tracing::debug!("ModuleManager start!");
        let module_list = Arc::new(Mutex::new(Vec::<Module>::new()));
        let instruct_module_list = module_list.clone();
        let manipulate_module_list = module_list.clone();

        let instruct_encoder = InstructEncoder::init(
            &summary_config.module_manager.encode_model_path,
            &summary_config.module_manager.encode_model_name,
        )?;
        let module_encoder = Arc::new(Mutex::new(instruct_encoder));
        let instruct_encoder = module_encoder.clone();

        let module_feature = Self::manager_module(module_list, module_receiver, module_encoder);

        let instruct_feature =
            Self::manager_instruct(instruct_module_list, instruct_receiver, instruct_encoder);

        let manipulate_feature =
            Self::manager_manipulate(manipulate_module_list, manipulate_receiver);

        tokio::try_join!(module_feature, instruct_feature, manipulate_feature)?;

        Ok(())
    }

    /// 处理操作的接收和转发
    ///
    /// 1、通过操作实体转发操作
    ///
    /// 2、记录日志
    ///
    /// 3、处理特定的错误
    async fn manager_manipulate(
        module_list: Arc<Mutex<Vec<Module>>>,
        mut manipulate_receiver: Receiver<ManipulateEntity>,
    ) -> Result<(), AppError> {
        tracing::debug!("manipulate_receiver start recv");
        while let Some(manipulate) = manipulate_receiver.recv().await {
            tracing::info!("get manipulate：{:?}", manipulate);
            if let Ok(mut modules) = module_list.lock() {
                for (_, module) in modules.iter_mut().enumerate() {
                    tracing::debug!("module name:{}", module.name)
                }
            } else {
                return Err(AppError::ModuleManagerError(
                    "Failed to obtain modules lock".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// 处理指令的接收和转发
    ///
    /// 1、通过模块索引找到处理对应指令的模块，将指令转发
    ///
    /// 2、记录操作到日志
    ///
    /// 3、转发指令出现错误时选择性重试
    async fn manager_instruct(
        module_list: Arc<Mutex<Vec<Module>>>,
        mut instruct_receiver: Receiver<InstructEntity>,
        instruct_encoder: Arc<Mutex<InstructEncoder>>,
    ) -> Result<(), AppError> {
        tracing::debug!("instruct_receiver start recv");
        while let Some(instruct) = instruct_receiver.recv().await {
            tracing::info!("from mpsc receiver get instruct：{:?}", &instruct);
            for message in instruct.message {
                if let Ok(mut encoder) = instruct_encoder.lock() {
                    let result = encoder.encode(message)?;
                    tracing::info!("encode result len: {}", result.len());
                    tracing::info!("encode result: {:?}", result)
                } else {
                    return Err(AppError::ModuleManagerError(
                        "Failed to obtain encoder lock".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// 负责管理子模块
    ///
    /// 1、定时获取子模块心跳，当离线时将对应子模块从模组中卸载
    ///
    /// 2、后续需要实现注册子模块的构建索引
    ///
    /// 3、特定错误进行重试或只通知
    async fn manager_module(
        module_list: Arc<Mutex<Vec<Module>>>,
        mut module_receiver: Receiver<Module>,
        instruct_encoder: Arc<Mutex<InstructEncoder>>,
    ) -> Result<(), AppError> {
        tracing::debug!("module_receiver start recv");
        while let Some(module) = module_receiver.recv().await {
            tracing::info!("register module：{:?}", &module.name);
            if let Ok(mut modules) = module_list.lock() {
                modules.push(module);
            } else {
                return Err(AppError::ModuleManagerError(
                    "Failed to obtain modules lock".to_string(),
                ));
            }
        }
        Ok(())
    }
}
