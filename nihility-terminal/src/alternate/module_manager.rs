use std::ops::Deref;
use std::sync::{Arc, Mutex};

use ndarray::{Array1, Axis, CowArray};
use ort::{
    Environment,
    ExecutionProvider,
    GraphOptimizationLevel,
    Session,
    SessionBuilder,
    Value
};
use ort::tensor::OrtOwnedTensor;
use tokenizers::Tokenizer;
use tokio::sync::mpsc::Receiver;

use crate::alternate::{
    config::SummaryConfig,
    module::{
        InstructEntity,
        ManipulateEntity,
        Module,
    },
};
use crate::error::AppError;

pub struct ModuleManager;

pub struct InstructEncoder {
    ort_session: Session,
    tokenizer: Tokenizer,
}

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

        let instruct_encoder = InstructEncoder::init(&summary_config.module_manager.encode_model_path, &summary_config.module_manager.encode_model_name)?;
        let module_encoder = Arc::new(Mutex::new(instruct_encoder));
        let instruct_encoder = module_encoder.clone();

        let module_feature =  Self::manager_module(module_list, module_receiver, module_encoder);

        let instruct_feature =  Self::manager_instruct(instruct_module_list, instruct_receiver, instruct_encoder);

        let manipulate_feature = Self::manager_manipulate(manipulate_module_list, manipulate_receiver);

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
        mut manipulate_receiver: Receiver<ManipulateEntity>
    ) -> Result<(), AppError> {
        tracing::debug!("manipulate_receiver start recv");
        while let Some(manipulate) = manipulate_receiver.recv().await {
            tracing::info!("get manipulate：{:?}", manipulate);
            if let Ok(mut modules) = module_list.lock() {
                for (_, module) in modules.iter_mut().enumerate() {
                    tracing::debug!("module name:{}", module.name)
                }
            } else {
                return Err(AppError::ModuleManagerError("Failed to obtain modules lock".to_string()))
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
        instruct_encoder: Arc<Mutex<InstructEncoder>>
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
                    return Err(AppError::ModuleManagerError("Failed to obtain encoder lock".to_string()))
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
        instruct_encoder: Arc<Mutex<InstructEncoder>>
    ) -> Result<(), AppError> {
        tracing::debug!("module_receiver start recv");
        while let Some(module) = module_receiver.recv().await {
            tracing::info!("register module：{:?}", &module.name);
            if let Ok(mut modules) = module_list.lock() {
                modules.push(module);
            } else {
                return Err(AppError::ModuleManagerError("Failed to obtain modules lock".to_string()))
            }
        }
        Ok(())
    }
}

impl InstructEncoder {
    pub fn init(
        model_path: &String,
        model_name: &String
    ) -> Result<InstructEncoder, AppError>  {
        let onnx_model_path = format!("{}/{}/model.onnx", model_path, model_name);
        let tokenizers_config_path = format!("{}/{}/tokenizer.json", model_path, model_name);
        tracing::debug!("onnx_model_path:{}", &onnx_model_path);
        tracing::debug!("tokenizers_config_path:{}", &tokenizers_config_path);
        let environment = Environment::builder()
            .with_name("test")
            .with_execution_providers([ExecutionProvider::CPU(Default::default())])
            .build()?
            .into_arc();
        let session = SessionBuilder::new(&environment)?
            .with_optimization_level(GraphOptimizationLevel::Level1)?
            .with_model_from_file(onnx_model_path)?;

        let tokenizer = Tokenizer::from_file(tokenizers_config_path).unwrap();

        Ok(InstructEncoder {
            ort_session: session,
            tokenizer
        })
    }

    pub fn encode(
        &mut self,
        input: String
    ) -> Result<Vec<f32>, AppError> {
        let encoding = self.tokenizer.encode(input, false).unwrap();
        tracing::debug!("encoding: {:?}", &encoding);

        let ids = encoding.get_ids().iter().map(|i| *i as i64).collect::<Vec<_>>();
        let mut ids = CowArray::from(Array1::from_iter(ids.iter().cloned()));
        let n_ids = ids.shape()[0];
        let input_ids = ids.clone().insert_axis(Axis(0)).into_shape((1, n_ids)).unwrap().into_dyn();

        let type_ids = encoding.get_type_ids().iter().map(|i| *i as i64).collect::<Vec<_>>();
        let mut type_ids = CowArray::from(Array1::from_iter(type_ids.iter().cloned()));
        let n_type_ids = type_ids.shape()[0];
        let token_type_ids = type_ids.clone().insert_axis(Axis(0)).into_shape((1, n_type_ids)).unwrap().into_dyn();

        let attention_mask = encoding.get_attention_mask().iter().map(|i| *i as i64).collect::<Vec<_>>();
        let mut attention_mask = CowArray::from(Array1::from_iter(attention_mask.iter().cloned()));
        let n_attention_mask = attention_mask.shape()[0];
        let attention_mask = attention_mask.clone().insert_axis(Axis(0)).into_shape((1, n_attention_mask)).unwrap().into_dyn();

        let inputs = vec![Value::from_array(self.ort_session.allocator(), &input_ids)?,
                          Value::from_array(self.ort_session.allocator(), &token_type_ids)?,
                          Value::from_array(self.ort_session.allocator(), &attention_mask)?,];
        tracing::debug!("onnx inputs: {:?}", &inputs);
        let outputs: Vec<Value> = self.ort_session.run(inputs)?;
        let generated_tokens: OrtOwnedTensor<f32, _> = outputs[0].try_extract()?;

        if let Some(result) = generated_tokens.view().deref().to_slice() {
            let result = result.iter().cloned().into_iter().collect::<Vec<f32>>();
            return Ok(result)
        } else {
            return Err(AppError::ModuleManagerError("encode error".to_string()))
        }
    }
}
