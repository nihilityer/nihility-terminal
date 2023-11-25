use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use tracing::info;

use crate::config::{EncoderConfig, EncoderType};

mod sentence_transformers;

/// 所有指令编码模块全部实现此特征
pub trait Encoder {
    /// 初始化编码模块
    fn init(model_path: String, model_name: String) -> Result<Self>
    where
        Self: Sized + Send;

    /// 对指令字符串进行编码
    fn encode(&mut self, input: String) -> Result<Vec<f32>>;

    /// 获取编码模块编码结果长度
    fn encode_size(&self) -> u64;
}

pub fn encoder_builder(
    encoder_config: &EncoderConfig,
) -> Result<Arc<Mutex<Box<dyn Encoder + Send>>>> {
    info!("use encoder type: {:?}", &encoder_config.encoder_type);
    return match encoder_config.encoder_type {
        EncoderType::SentenceTransformers => {
            let encoder = sentence_transformers::SentenceTransformers::init(
                encoder_config.model_path.to_string(),
                encoder_config.model_name.to_string(),
            )?;
            Ok(Arc::new(Mutex::new(Box::new(encoder))))
        }
    };
}
