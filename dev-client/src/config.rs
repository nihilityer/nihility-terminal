use std::error::Error;
use serde::{Deserialize, Serialize};
use serde_json;
use local_ip_address::local_ip;
use figment::{
    providers::{Format, Json, Serialized},
    Figment
};
use std::{
    path::Path,
    fs::File,
    io::Write
};

#[derive(Deserialize, Serialize)]
pub struct ClientConfig {
    pub log: LogConfig,
    pub grpc: GrpcConfig,
    pub pipe: PipeConfig,
    pub multicast: MulticastConfig,
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
    pub multicast_group: String,
    pub multicast_port: u32,
    pub multicast_info: String,
    pub interval: u32,
}

impl ClientConfig {
    fn default() -> Result<Self, Box<dyn Error>> {
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
            directory: "../nihility-terminal/communication".to_string(),
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

        let multicast_info = format!("{}:{}", &grpc_config.addr, &grpc_config.port);
        let multicast_config = MulticastConfig {
            enable: false,
            bind_addr: "0.0.0.0".to_string(),
            multicast_group: "224.0.0.123".to_string(),
            multicast_port: 1234,
            multicast_info,
            interval: 5,
        };

        Ok(ClientConfig {
            log: log_config,
            grpc: grpc_config,
            pipe: pipe_config,
            multicast: multicast_config,
        })
    }

    pub fn init() -> Result<Self, Box<dyn Error>> {

        let mut config = ClientConfig::default()?;

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
        let result: ClientConfig = Figment::from(Serialized::defaults(config))
            .merge(Json::file("config.json"))
            .extract()?;
        Ok(result)
    }
}