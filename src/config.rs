use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use figment::providers::{Format, Json, Serialized, Toml, Yaml};
use figment::Figment;
use nihility_common::GrpcServerConfig;
use serde::{Deserialize, Serialize};

use crate::core::instruct_encoder::sentence_transformers;
use crate::core::instruct_encoder::sentence_transformers::{MODULE_NAME, MODULE_PATH};
use crate::core::instruct_matcher::grpc_qdrant::QDRANT_GRPC_ADDR_FIELD;
use crate::core::instruct_matcher::ENCODE_SIZE_FIELD;

const JSON_CONFIG_FILE_NAME: &str = "config.json";
const TOML_CONFIG_FILE_NAME: &str = "config.toml";
const YAML_CONFIG_FILE_NAME: &str = "config.yaml";

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct SummaryConfig {
    pub log: LogConfig,
    pub server: ServerConfig,
    pub core: CoreConfig,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LogConfig {
    pub enable: bool,
    pub level: String,
    pub with_file: bool,
    pub with_line_number: bool,
    pub with_thread_ids: bool,
    pub with_target: bool,
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
}

#[derive(Deserialize, Serialize, PartialEq, Default, Debug, Clone)]
pub enum HeartbeatManagerType {
    #[default]
    Simple,
}

#[derive(Deserialize, Serialize, PartialEq, Default, Debug, Clone)]
pub enum InstructManagerType {
    #[default]
    Simple,
}

#[derive(Deserialize, Serialize, PartialEq, Default, Debug, Clone)]
pub enum ManipulateManagerType {
    #[default]
    Simple,
}

#[derive(Deserialize, Serialize, PartialEq, Default, Debug, Clone)]
pub enum SubmoduleManagerType {
    #[default]
    Simple,
}

#[derive(Deserialize, Serialize, PartialEq, Default, Debug, Clone)]
pub enum InstructEncoderType {
    #[default]
    SentenceTransformers,
}

#[derive(Deserialize, Serialize, PartialEq, Default, Debug, Clone)]
pub enum InstructMatcherType {
    #[default]
    GrpcQdrant,
}

#[derive(Deserialize, Serialize, PartialEq, Default, Debug, Clone)]
pub enum SubmoduleStoreType {
    #[default]
    SimpleHashMap,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct InstructEncoderConfig {
    pub instruct_encoder_type: InstructEncoderType,
    pub config_map: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct InstructMatcherConfig {
    pub instruct_matcher_type: InstructMatcherType,
    pub config_map: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct SubmoduleStoreConfig {
    pub submodule_store_type: SubmoduleStoreType,
    pub config_map: HashMap<String, String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig {
            enable: true,
            level: "INFO".to_string(),
            with_file: false,
            with_line_number: false,
            with_thread_ids: false,
            with_target: false,
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
            config_map,
        }
    }
}

impl Default for InstructMatcherConfig {
    fn default() -> Self {
        let mut config_map = HashMap::<String, String>::new();
        config_map.insert(
            QDRANT_GRPC_ADDR_FIELD.to_string(),
            "http://192.168.0.100:6334".to_string(),
        );
        config_map.insert(
            ENCODE_SIZE_FIELD.to_string(),
            sentence_transformers::ENCODE_SIZE.to_string(),
        );
        InstructMatcherConfig {
            instruct_matcher_type: InstructMatcherType::default(),
            config_map,
        }
    }
}

impl SummaryConfig {
    pub fn init() -> Result<Self> {
        let config = SummaryConfig::default();
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
            let mut config_file: File = File::create(TOML_CONFIG_FILE_NAME)?;
            config_file.write_all(toml::to_string_pretty(&config)?.as_bytes())?;
            config_file.flush()?;
            Ok(config)
        }
    }
}
