use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use figment::providers::{Format, Json, Serialized, Toml, Yaml};
use figment::Figment;
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

const JSON_CONFIG_FILE_NAME: &str = "config.json";
const TOML_CONFIG_FILE_NAME: &str = "config.toml";
const YAML_CONFIG_FILE_NAME: &str = "config.yaml";

/// 总配置
#[derive(Deserialize, Serialize)]
pub struct SummaryConfig {
    pub log: LogConfig,
    pub communicat: CommunicatConfig,
    pub core: CoreConfig,
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

#[derive(Deserialize, Serialize, Clone)]
pub struct CommunicatConfig {
    pub grpc: GrpcConfig,
    #[cfg(unix)]
    pub pipe: PipeConfig,
    #[cfg(windows)]
    pub windows_named_pipes: WindowsNamedPipesConfig,
    pub multicast: MulticastConfig,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct CoreConfig {
    pub module_manager: InstructManagerConfig,
    pub encoder: EncoderConfig,
    pub channel_buffer: usize,
}


/// Grpc相关配置
#[derive(Deserialize, Serialize, Clone)]
pub struct GrpcConfig {
    pub enable: bool,
    pub addr: String,
    pub port: u32,
}

/// unix管道通信相关配置
///
/// 注：仅在unix系统上支持
#[derive(Deserialize, Serialize, Clone)]
#[cfg(unix)]
pub struct PipeConfig {
    pub enable: bool,
    pub directory: String,
    pub module: String,
    pub instruct_receiver: String,
    pub manipulate_receiver: String,
}

/// windows管道通信相关配置
#[derive(Deserialize, Serialize, Clone)]
#[cfg(windows)]
pub struct WindowsNamedPipesConfig {
    pub enable: bool,
    pub pipe_prefix: String,
    pub module_pipe_name: String,
    pub instruct_pipe_name: String,
    pub manipulate_pipe_name: String,
}

/// 组播相关配置
#[derive(Deserialize, Serialize, Clone)]
pub struct MulticastConfig {
    pub enable: bool,
    pub bind_addr: String,
    pub bind_port: u32,
    pub multicast_group: String,
    pub multicast_port: u32,
    pub multicast_info: String,
    pub interval: u32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum InstructManagerType {
    GrpcQdrant,
}

/// 子模块管理相关配置（核心配置）
///
/// 目前没有多少能正常配置的
#[derive(Deserialize, Serialize, Clone)]
pub struct InstructManagerConfig {
    pub manager_type: InstructManagerType,
    pub config_map: HashMap<String, String>,
}

/// 指令编码模块配置
#[derive(Deserialize, Serialize, Clone)]
pub struct EncoderConfig {
    pub encoder_type: String,
    pub model_path: String,
    pub model_name: String,
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
            module: "model".to_string(),
            instruct_receiver: "instruct_receiver".to_string(),
            manipulate_receiver: "manipulate_receiver".to_string(),
        };

        #[cfg(windows)]
        let windows_named_pipes_config = WindowsNamedPipesConfig {
            enable: true,
            pipe_prefix: r"\\.\pipe\nihilityer".to_string(),
            module_pipe_name: "model".to_string(),
            instruct_pipe_name: "master_instruct".to_string(),
            manipulate_pipe_name: "manipulate".to_string(),
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

        let mut config_map = HashMap::<String, String>::new();
        config_map.insert(
            "qdrant_grpc_addr".to_string(),
            "http://192.168.0.100:6334".to_string(),
        );
        let module_manager_config = InstructManagerConfig {
            manager_type: InstructManagerType::GrpcQdrant,
            config_map,
        };

        let encoder_config = EncoderConfig {
            encoder_type: "sentence_transformers".to_string(),
            model_path: "model".to_string(),
            model_name: "onnx_bge_small_zh".to_string(),
        };

        let core_config = CoreConfig {
            module_manager: module_manager_config,
            encoder: encoder_config,
            channel_buffer: 10,
        };

        #[cfg(windows)]
        let communicat_config = CommunicatConfig {
            grpc: grpc_config,
            windows_named_pipes: windows_named_pipes_config,
            multicast: multicast_config,
        };

        #[cfg(unix)]
        let communicat_config = CommunicatConfig {
            grpc: grpc_config,
            pipe: pipe_config,
            multicast: multicast_config,
        };

        return Ok(SummaryConfig {
            log: log_config,
            core: core_config,
            communicat: communicat_config,
        });
    }

    /// 当配置文件不存在时使用默认配置当配置文件不存在时使用默认配置
    pub fn init() -> Result<Self, AppError> {
        let mut config = SummaryConfig::default()?;

        return if Path::try_exists(TOML_CONFIG_FILE_NAME.as_ref())? {
            let result: SummaryConfig = Figment::from(Serialized::defaults(config))
                .merge(Toml::file(TOML_CONFIG_FILE_NAME))
                .extract()?;
            Ok(result)
        } else if Path::try_exists(YAML_CONFIG_FILE_NAME.as_ref())? {
            let result: SummaryConfig = Figment::from(Serialized::defaults(config))
                .merge(Yaml::file(YAML_CONFIG_FILE_NAME))
                .extract()?;
            Ok(result)
        } else if Path::try_exists(JSON_CONFIG_FILE_NAME.as_ref())? {
            let result: SummaryConfig = Figment::from(Serialized::defaults(config))
                .merge(Json::file(JSON_CONFIG_FILE_NAME))
                .extract()?;
            Ok(result)
        } else {
            config.communicat.grpc.enable = false;
            config.communicat.multicast.enable = false;

            let mut config_file = File::create(TOML_CONFIG_FILE_NAME)?;
            config_file.write_all(toml::to_string_pretty(&config)?.as_bytes())?;
            config_file.flush()?;
            Ok(config)
        };
    }
}
