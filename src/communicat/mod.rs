use async_trait::async_trait;
use nihility_common::instruct::InstructReq;
use nihility_common::manipulate::ManipulateReq;
use nihility_common::response_code::RespCode;

use crate::AppError;

pub mod grpc;
#[cfg(unix)]
pub mod pipe;
#[cfg(windows)]
pub mod windows_named_pipe;
pub mod multicast;

/// 发送指令特征
#[async_trait]
pub trait SendInstructOperate {
    /// 发送指令
    async fn send(&mut self, instruct: InstructReq) -> Result<RespCode, AppError>;
}

/// 发送操作特征
#[async_trait]
pub trait SendManipulateOperate {
    /// 发送操作
    async fn send(&mut self, manipulate: ManipulateReq) -> Result<RespCode, AppError>;
}
