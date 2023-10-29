use std::sync::Arc;
use std::sync::Mutex;

use crate::config::EncoderConfig;
use crate::AppError;

mod sentence_transformers;

/// 所有指令编码模块全部实现此特征
pub trait Encoder {
    /// 初始化编码模块
    fn init(model_path: String, model_name: String) -> Result<Self, AppError> where Self: Sized + Send;

    /// 对指令字符串进行编码
    fn encode(&mut self, input: String) -> Result<Vec<f32>, AppError>;
}

pub fn encoder_builder(
    encoder_config: &EncoderConfig,
) -> Result<Arc<Mutex<Box<dyn Encoder + Send>>>, AppError> {
    return match encoder_config.encoder_type.to_lowercase().as_str() {
        "sentence_transformers" => {
            let encoder = sentence_transformers::SentenceTransformers::init(
                encoder_config.model_path.to_string(),
                encoder_config.model_name.to_string(),
            )?;
            Ok(Arc::new(Mutex::new(Box::new(encoder))))
        }
        _ => Err(AppError::EncoderError("init".to_string())),
    };
}
