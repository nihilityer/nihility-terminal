use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use figment::providers::{Format, Json, Serialized, Toml, Yaml};
use figment::Figment;
use nihility_common::{GrpcServerConfig, LogConfig};
use serde::{Deserialize, Serialize};

use crate::core::instruct_encoder::sentence_transformers::{MODULE_NAME, MODULE_PATH};

const JSON_CONFIG_FILE_NAME: &str = "config.json";
const TOML_CONFIG_FILE_NAME: &str = "config.toml";
const YAML_CONFIG_FILE_NAME: &str = "config.yaml";

#[cfg(target_os = "windows")]
pub const ORT_LIB_PATH: &str = "lib/onnxruntime.dll";
#[cfg(any(target_os = "linux", target_os = "android"))]
pub const ORT_LIB_PATH: &str = "lib/libonnxruntime.so";
#[cfg(target_os = "macos")]
pub const ORT_LIB_PATH: &str = "lib/libonnxruntime.dylib";

#[derive(Deserialize, Serialize, Debug)]
pub struct NihilityTerminalConfig {
    pub log: Vec<LogConfig>,
    pub server: ServerConfig,
    pub core: CoreConfig,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct ServerConfig {
    pub grpc_server: GrpcServerConfig,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct CoreConfig {
    pub heartbeat_manager: HeartbeatManagerType,
    pub instruct_manager: InstructManagerType,
    pub manipulate_manager: ManipulateManagerType,
    pub submodule_manager: SubmoduleManagerType,
    pub instruct_matcher: InstructMatcherConfig,
    pub instruct_encoder: InstructEncoderConfig,
    pub submodule_store: SubmoduleStoreConfig,
    pub auth_key_dir: String,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub enum HeartbeatManagerType {
    #[default]
    Simple,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub enum InstructManagerType {
    #[default]
    Simple,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub enum ManipulateManagerType {
    #[default]
    Simple,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub enum SubmoduleManagerType {
    #[default]
    Simple,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub enum InstructEncoderType {
    #[default]
    SentenceTransformers,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub enum InstructMatcherType {
    GrpcQdrant,
    #[default]
    InstantDistance,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub enum SubmoduleStoreType {
    #[default]
    SimpleHashMap,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct InstructEncoderConfig {
    pub instruct_encoder_type: InstructEncoderType,
    pub ort_lib_path: String,
    pub config_map: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct InstructMatcherConfig {
    pub instruct_matcher_type: InstructMatcherType,
    pub config_map: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct SubmoduleStoreConfig {
    pub submodule_store_type: SubmoduleStoreType,
    pub config_map: HashMap<String, String>,
}

impl Default for NihilityTerminalConfig {
    fn default() -> Self {
        NihilityTerminalConfig {
            log: vec![LogConfig::default()],
            server: ServerConfig::default(),
            core: CoreConfig::default(),
        }
    }
}

impl Default for InstructEncoderConfig {
    fn default() -> Self {
        let mut config_map = HashMap::<String, String>::new();
        config_map.insert(MODULE_PATH.to_string(), String::from("model"));
        config_map.insert(MODULE_NAME.to_string(), String::from("onnx_bge_small_zh"));
        InstructEncoderConfig {
            instruct_encoder_type: InstructEncoderType::default(),
            ort_lib_path: ORT_LIB_PATH.to_string(),
            config_map,
        }
    }
}

impl NihilityTerminalConfig {
    pub fn init() -> Result<Self> {
        let config = NihilityTerminalConfig::default();
        if Path::try_exists(TOML_CONFIG_FILE_NAME.as_ref())? {
            Ok(Figment::merge(
                Figment::from(Serialized::defaults(config)),
                Toml::file(TOML_CONFIG_FILE_NAME),
            )
            .extract()?)
        } else if Path::try_exists(YAML_CONFIG_FILE_NAME.as_ref())? {
            Ok(Figment::from(Serialized::defaults(config))
                .merge(Yaml::file(YAML_CONFIG_FILE_NAME))
                .extract()?)
        } else if Path::try_exists(JSON_CONFIG_FILE_NAME.as_ref())? {
            Ok(Figment::from(Serialized::defaults(config))
                .merge(Json::file(JSON_CONFIG_FILE_NAME))
                .extract()?)
        } else {
            let mut config_file: File = File::create(JSON_CONFIG_FILE_NAME)?;
            config_file.write_all(serde_json::to_string_pretty(&config)?.as_bytes())?;
            config_file.flush()?;
            Ok(config)
        }
    }
}
