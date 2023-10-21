use std::fs::File;
use std::io::Write;
use std::path::Path;

use figment::Figment;
use figment::providers::{Format, Json, Serialized};
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// 总配置
#[derive(Deserialize, Serialize)]
pub struct SummaryConfig {
    pub log: LogConfig,
    pub grpc: GrpcConfig,
    #[cfg(unix)]
    pub pipe: PipeConfig,
    #[cfg(windows)]
    pub windows_pipe: PipeWindowsConfig,
    pub multicast: MulticastConfig,
    pub module_manager: ModuleManagerConfig,
}

/// 日志相关配置
#[derive(Deserialize, Serialize)]
pub struct LogConfig {
    pub enable: bool,
    pub level: String,
    pub with_file: bool,
    pub with_line_number: bool,
    pub with_thread_ids: bool,
    pub with_target: bool,
}

/// Grpc相关配置
#[derive(Deserialize, Serialize)]
pub struct GrpcConfig {
    pub enable: bool,
    pub addr: String,
    pub port: u32,
}

/// unix管道通信相关配置
#[derive(Deserialize, Serialize)]
#[cfg(unix)]
pub struct PipeConfig {
    pub enable: bool,
    pub directory: String,
    pub module: String,
    pub instruct_receiver: String,
    pub instruct_sender: String,
    pub manipulate_receiver: String,
    pub manipulate_sender: String,
}

/// windows管道通信相关配置
#[derive(Deserialize, Serialize)]
pub struct PipeWindowsConfig {
    // TODO
    pub enable: bool,
    pub addr: String,
    pub port: u32,
}

/// 组播相关配置
#[derive(Deserialize, Serialize)]
pub struct MulticastConfig {
    pub enable: bool,
    pub bind_addr: String,
    pub bind_port: u32,
    pub multicast_group: String,
    pub multicast_port: u32,
    pub multicast_info: String,
    pub interval: u32,
}

/// 子模块管理相关配置（核心配置）
///
/// 目前没有多少能正常配置的
#[derive(Deserialize, Serialize)]
pub struct ModuleManagerConfig {
    pub interval: u32,
    pub channel_buffer: usize,
    pub encode_model_path: String,
    pub encode_model_name: String,
}

impl SummaryConfig {
    fn default() -> Result<Self, AppError> {
        let local_ip_addr = local_ip()?;

        let log_config = LogConfig {
            enable: true,
            level: "INFO".to_string(),
            with_file: false,
            with_line_number: false,
            with_thread_ids: false,
            with_target: false,
        };

        let grpc_config = GrpcConfig {
            enable: false,
            addr: local_ip_addr.to_string(),
            port: 5050,
        };

        #[cfg(unix)]
        let work_dir = std::fs::canonicalize("../")?
            .to_str()
            .ok_or(AppError::ConfigError("create workdir config".to_string()))?
            .to_string();
        #[cfg(unix)]
        let work_dir = format!("{}/communication", work_dir);
        #[cfg(unix)]
        let pipe_config = PipeConfig {
            enable: true,
            directory: work_dir,
            module: "module".to_string(),
            instruct_receiver: "instruct_receiver".to_string(),
            instruct_sender: "instruct_sender".to_string(),
            manipulate_receiver: "manipulate_receiver".to_string(),
            manipulate_sender: "manipulate_sender".to_string(),
        };

        #[cfg(windows)]
        let pipe_windows_config = PipeWindowsConfig {
            enable: true,
            addr: "todo".to_string(),
            port: 1111,
        };

        let mut multicast_info = grpc_config.addr.to_string();
        multicast_info.push_str(format!(":{}", &grpc_config.port).as_str());
        let multicast_config = MulticastConfig {
            enable: false,
            bind_addr: "0.0.0.0".to_string(),
            bind_port: 0,
            multicast_group: "224.0.0.123".to_string(),
            multicast_port: 1234,
            multicast_info,
            interval: 5,
        };

        let module_manager_config = ModuleManagerConfig {
            interval: 1,
            channel_buffer: 10,
            encode_model_path: "model".to_string(),
            encode_model_name: "onnx_bge_small_zh".to_string(),
        };

        #[cfg(unix)]
        return Ok(SummaryConfig {
            log: log_config,
            grpc: grpc_config,
            pipe: pipe_config,
            multicast: multicast_config,
            module_manager: module_manager_config,
        });
        #[cfg(windows)]
        return Ok(SummaryConfig {
            log: log_config,
            grpc: grpc_config,
            windows_pipe: pipe_windows_config,
            multicast: multicast_config,
            module_manager: module_manager_config,
        });
    }

    /// 当配置文件不存在时使用默认配置当配置文件不存在时使用默认配置
    pub fn init() -> Result<Self, AppError> {
        let mut config = SummaryConfig::default()?;

        if !Path::try_exists("config.json".as_ref())? {
            println!("can not find config");
            config.grpc.enable = false;
            config.multicast.enable = false;

            let mut config_file = File::create("config.json")?;

            config_file.write_all(serde_json::to_string_pretty(&config)?.as_bytes())?;
            config_file.flush()?;

            return Ok(config);
        }
        let result: SummaryConfig = Figment::from(Serialized::defaults(config))
            .merge(Json::file("config.json"))
            .extract()?;
        Ok(result)
    }
}
