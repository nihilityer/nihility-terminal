use anyhow::Result;
use crate::config::InstructEncoderConfig;

pub mod sentence_transformers;

/// 所有指令编码模块全部实现此特征
pub trait InstructEncoder {
    /// 初始化编码模块
    fn init(instruct_encoder_config: &InstructEncoderConfig) -> Result<Self>
    where
        Self: Sized + Send + Sync;

    /// 对指令字符串进行编码
    fn encode(&self, input: &String) -> Result<Vec<f32>>;

    /// 获取编码模块编码结果长度
    fn encode_size(&self) -> u64;
}
