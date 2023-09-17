use std::{
    path::Path,
    fs::File,
    io::Write
};
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use serde_json;
use figment::{
    providers::{Format, Json, Serialized},
    Figment
};
use crate::error::AppError;

#[derive(Deserialize, Serialize)]
pub struct SummaryConfig {
    pub log: LogConfig,
    pub grpc: GrpcConfig,
    pub pipe: PipeConfig,
    pub multicast: MulticastConfig,
    pub module_manager: ModuleManagerConfig,
}

#[derive(Deserialize, Serialize)]
pub struct LogConfig {
    pub enable: bool,
    pub level: String,
    pub with_file: bool,
    pub with_line_number: bool,
    pub with_thread_ids: bool,
    pub with_target: bool,
}

#[derive(Deserialize, Serialize)]
pub struct GrpcConfig {
    pub enable: bool,
    pub addr: String,
    pub port: u32,
}

#[derive(Deserialize, Serialize)]
pub struct PipeConfig {
    pub enable: bool,
    pub unix: PipeUnixConfig,
    pub windows: PipeWindowsConfig,
}

#[derive(Deserialize, Serialize)]
pub struct PipeUnixConfig {
    pub directory: String,
    pub module: String,
    pub instruct_receiver: String,
    pub instruct_sender: String,
    pub manipulate_receiver: String,
    pub manipulate_sender: String,
}

#[derive(Deserialize, Serialize)]
pub struct PipeWindowsConfig {
    // TODO
    pub addr: String,
    pub port: u32,
}

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

#[derive(Deserialize, Serialize)]
pub struct ModuleManagerConfig {
    pub interval: u32,
    pub channel_buffer: usize,
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

        let pipe_unix_config = PipeUnixConfig {
            directory: "communication".to_string(),
            module: "module".to_string(),
            instruct_receiver: "instruct_receiver".to_string(),
            instruct_sender: "instruct_sender".to_string(),
            manipulate_receiver: "manipulate_receiver".to_string(),
            manipulate_sender: "manipulate_sender".to_string(),
        };

        let pipe_windows_config = PipeWindowsConfig {
            addr: "todo".to_string(),
            port: 1111,
        };

        let pipe_config = PipeConfig {
            enable: true,
            unix: pipe_unix_config,
            windows: pipe_windows_config,
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
        };

        Ok(SummaryConfig {
            log: log_config,
            grpc: grpc_config,
            pipe: pipe_config,
            multicast: multicast_config,
            module_manager: module_manager_config,
        })
    }

    pub fn init() -> Result<Self, AppError> {

        let mut config = SummaryConfig::default()?;

        if !Path::try_exists("config.json".as_ref())? {
            println!("未找到配置文件，开始使用默认设置");
            config.pipe.enable = true;
            config.grpc.enable = false;
            config.multicast.enable = false;

            let mut config_file = File::create("config.json")?;

            config_file.write_all(serde_json::to_string_pretty(&config)?.as_bytes())?;
            config_file.flush()?;

            return Ok(config)
        }
        let result: SummaryConfig = Figment::from(Serialized::defaults(config))
            .merge(Json::file("config.json"))
            .extract()?;
        println!("配置初始化成功：Grpc：{}，pipe：{}，multicast：{}", &result.grpc.enable, &result.pipe.enable, &result.multicast.enable);
        Ok(result)
    }
}
