use thiserror::Error;

#[derive(Error, Debug)]
pub enum  AppError {
    #[error("应用{0}模块配置错误！")]
    ConfigError(String),

    #[error("配置初始化错误！")]
    ConfigInitError(#[from] figment::Error),

    #[error("获取本机Ip错误！")]
    GetLocalIpError(#[from] local_ip_address::Error),

    #[error("模块管理中{0}错误！")]
    ModuleManagerError(String),

    #[error("日志模块初始化异常！")]
    LogInitException(#[from] tracing::dispatcher::SetGlobalDefaultError),

    #[error("网络地址转换异常！")]
    AddrException(#[from] std::net::AddrParseError),

    #[error("Grpc服务启动异常！")]
    GrpcServerStartException(#[from] tonic::transport::Error),

    #[error("Json转换异常！")]
    JsonException(#[from] serde_json::Error),

    #[error("系统IO异常！")]
    SystemIOException(#[from] std::io::Error),

    #[error("未知内部错误！")]
    OtherError,
}