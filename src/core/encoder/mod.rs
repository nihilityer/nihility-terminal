use anyhow::Result;
use tracing::info;

use EncoderType::SentenceTransformers;

use crate::config::{EncoderConfig, EncoderType};

pub mod mock;
mod sentence_transformers;

/// 所有指令编码模块全部实现此特征
pub trait InstructEncoder {
    /// 初始化编码模块
    fn init(model_path: String, model_name: String) -> Result<Self>
    where
        Self: Sized + Send + Sync;

    /// 对指令字符串进行编码
    fn encode(&mut self, input: String) -> Result<Vec<f32>>;

    /// 获取编码模块编码结果长度
    fn encode_size(&self) -> u64;
}

pub fn encoder_builder(
    encoder_config: &EncoderConfig,
) -> Result<Box<dyn InstructEncoder + Send + Sync>> {
    info!("Use Encoder Type: {:?}", &encoder_config.encoder_type);
    match encoder_config.encoder_type {
        SentenceTransformers => {
            let encoder = sentence_transformers::SentenceTransformers::init(
                encoder_config.model_path.to_string(),
                encoder_config.model_name.to_string(),
            )?;
            Ok(Box::new(encoder))
        }
    }
}
