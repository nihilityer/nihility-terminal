use thiserror::Error;

use crate::entity;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("应用{0}模块配置错误！")]
    ConfigError(String),

    #[error("配置初始化错误！")]
    ConfigInitError(#[from] figment::Error),

    #[error("获取本机Ip错误！")]
    GetLocalIpError(#[from] local_ip_address::Error),

    #[error("模块管理中{0}错误！")]
    ModuleManagerError(String),

    #[error("编码模块：{0}错误！")]
    EncoderError(String),

    #[error("管道通讯{0}错误！")]
    PipeError(String),

    #[error("Module Mpsc unexpected shutdown")]
    ModuleMpscMessageError(#[from] tokio::sync::mpsc::error::SendError<entity::module::Module>),

    #[error("Instruct Mpsc unexpected shutdown")]
    InstructMpscMessageError(
        #[from] tokio::sync::mpsc::error::SendError<entity::instruct::InstructEntity>,
    ),

    #[error("Manipulate Mpsc unexpected shutdown")]
    ManipulateMpscMessageError(
        #[from] tokio::sync::mpsc::error::SendError<entity::manipulate::ManipulateEntity>,
    ),

    #[error("Tonic错误！")]
    TonicError(#[from] tonic::Status),

    #[error("{0}枚举转换错误！")]
    ProstTransferError(String),

    #[error("prost 解码错误！")]
    DecodeError(#[from] prost::DecodeError),

    #[error("prost 编码错误！")]
    ProstEncodeError(#[from] prost::EncodeError),

    #[error("指令编码模块错误！{0}")]
    OrtError(#[from] ort::OrtError),

    #[error("操作系统不支持！")]
    OsNotSupportError,

    #[error("日志模块初始化异常！")]
    LogInitException(#[from] tracing::dispatcher::SetGlobalDefaultError),

    #[error("网络地址转换异常！")]
    AddrException(#[from] std::net::AddrParseError),

    #[error("Grpc服务启动异常！")]
    GrpcServerStartException(#[from] tonic::transport::Error),

    #[error("Grpc Module {0} get a exception")]
    GrpcModuleException(String),

    #[error("Toml转换异常！")]
    JsonException(#[from] toml::ser::Error),

    #[error("系统IO异常！")]
    SystemIOException(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
