extern crate nihility_common;

use std::{
    net::Ipv4Addr,
    path::Path,
    process::Command,
    str::FromStr,
};
use std::error::Error;
use time::{macros::format_description, UtcOffset};

use tokio::{
    net::{
        UdpSocket,
        unix::pipe
    },
    time::{
        Duration
    },
    sync::oneshot,
};
use tonic::transport::Server;
use prost::Message;
use tracing::Level;

use grpc::{InstructImpl, ManipulateImpl};
use nihility_common::{
    instruct::{
        instruct_client::InstructClient,
        InstructReq
    },
    manipulate::{
        manipulate_client::ManipulateClient,
        ManipulateReq
    },
    module_info::{
        module_info_client::ModuleInfoClient,
        ModuleInfoReq,
    }
};
use nihility_common::instruct::instruct_server::InstructServer;
use nihility_common::instruct::InstructType;
use nihility_common::manipulate::manipulate_server::ManipulateServer;
use nihility_common::manipulate::ManipulateType;
use nihility_common::module_info::ClientType;
use crate::config::{ClientConfig, GrpcConfig, MulticastConfig};

mod grpc;
mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_config = ClientConfig::init()?;

    if client_config.log.enable {
        let mut subscriber = tracing_subscriber::fmt().compact();
        match client_config.log.level.as_str() {
            "debug" | "DEBUG" => {
                subscriber = subscriber.with_max_level(Level::DEBUG);
            }
            "info" | "INFO" => {
                subscriber = subscriber.with_max_level(Level::INFO);
            }
            "warn" | "WARN" => {
                subscriber = subscriber.with_max_level(Level::WARN);
            }
            "error" | "ERROR" => {
                subscriber = subscriber.with_max_level(Level::ERROR);
            }
            _ => {
                panic!("log error");
            }
        }

        let timer = tracing_subscriber::fmt::time::OffsetTime::new(
            UtcOffset::from_hms(8, 0, 0).unwrap(),
            format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"),
        );
        let subscriber = subscriber
            .with_line_number(client_config.log.with_line_number)
            .with_thread_ids(client_config.log.with_thread_ids)
            .with_target(client_config.log.with_target)
            .with_timer(timer)
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;
        tracing::info!("日志模块初始化完成！");
    }

    let (re, rx) = oneshot::channel::<String>();

    let multicast_future = multicast(&client_config.multicast, re);
    let server_future = grpc_server(&client_config.grpc);
    let register_future = register(&client_config, rx);

    let _ = tokio::try_join!(server_future, register_future, multicast_future);

    Ok(())
}

async fn multicast(multicast_config: &MulticastConfig, sender: oneshot::Sender<String>) -> Result<(), Box<dyn Error>> {
    if multicast_config.enable {
        let bind_addr = format!("{}:{}", &multicast_config.bind_addr, &multicast_config.multicast_port);
        let socket = UdpSocket::bind(bind_addr).await?;
        socket.join_multicast_v4(Ipv4Addr::from_str(&multicast_config.multicast_group.as_str())?, Ipv4Addr::new(0,0,0,0))?;
        // socket.connect("224.0.1.123:1234").await?;
        // socket.set_multicast_loop_v4(true)?;
        tracing::info!("socket:{:?}", &socket);
        let mut buf = [0u8; 1024];

        tracing::info!("开始接收信息！");
        let (count, _) = socket.recv_from(&mut buf).await?;
        tracing::info!("count:{}", count);
        let result = String::from_utf8(Vec::from(&buf[..count])).unwrap();
        let server_addr = format!("http://{}", &result);
        tracing::info!("{}", &server_addr);
        sender.send(server_addr)?;
    }
    Ok(())
}

async fn register(client_config: &ClientConfig, receiver: oneshot::Receiver<String>) -> Result<(), Box<dyn Error>> {
    tracing::info!("register start!");
    tokio::time::sleep(Duration::from_secs(5)).await;

    if client_config.grpc.enable {
        let grpc_addr = format!("{}:{}", &client_config.grpc.addr, &client_config.grpc.port);

        let (module_addr, instruct_addr, manipulate_addr) = match receiver.await {
            Ok(addr) => {
                (addr.to_string(), addr.to_string(), addr.to_string())
            },
            Err(_) => {
                panic!()
            }
        };
        let mut module_info_client = ModuleInfoClient::connect(module_addr).await.unwrap();
        let mut instruct_client = InstructClient::connect(instruct_addr).await.unwrap();
        let mut manipulate_client = ManipulateClient::connect(manipulate_addr).await.unwrap();

        let module_req = tonic::Request::new(ModuleInfoReq {
            name: "dev_client".into(),
            client_type: ClientType::GrpcType.into(),
            addr: vec![grpc_addr.to_string()]
        });
        let message = vec!["ce4".to_string(), "shi4".to_string()];
        let instruct_req = tonic::Request::new(InstructReq {
            instruct_type: InstructType::DefaultType.into(),
            message,
        });
        let manipulate_req = tonic::Request::new(ManipulateReq {
            manipulate_type: ManipulateType::DefaultType.into(),
            command: "test message".to_string(),
        });

        while let Err(_) = InstructClient::connect(grpc_addr.to_string()).await {
            tracing::info!("wait self server startup");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        let module_info_resp = module_info_client.register(module_req).await.unwrap();
        tracing::info!("module_info_resp:{:?}", module_info_resp.into_inner());
        let instruct_resp = instruct_client.send_instruct(instruct_req).await.unwrap();
        tracing::info!("instruct_resp:{:?}", instruct_resp.into_inner());
        let manipulate_resp = manipulate_client.send_manipulate(manipulate_req).await.unwrap();
        tracing::info!("manipulate_resp:{:?}", manipulate_resp.into_inner());
    }

    if client_config.pipe.enable {

        let module_path = format!("{}/{}", client_config.pipe.unix.directory.as_str(), client_config.pipe.unix.module.to_string());
        let instruct_path = format!("{}/{}", client_config.pipe.unix.directory.as_str(), client_config.pipe.unix.instruct_sender.to_string());
        let manipulate_path = format!("{}/{}", client_config.pipe.unix.directory.as_str(), client_config.pipe.unix.manipulate_sender.to_string());
        let client_instruct_path = format!("{}/{}", client_config.pipe.unix.directory, client_config.pipe.unix.instruct_receiver);
        let client_manipulate_path = format!("{}/{}", client_config.pipe.unix.directory, client_config.pipe.unix.manipulate_receiver);

        if !Path::try_exists(&client_instruct_path.as_ref())? {
            Command::new("mkfifo").arg(&client_instruct_path).spawn()?;
        }
        if !Path::try_exists(&client_manipulate_path.as_ref())? {
            Command::new("mkfifo").arg(&client_manipulate_path).spawn()?;
        }

        loop {
            if !Path::try_exists(&client_instruct_path.as_ref())? || !Path::try_exists(&client_manipulate_path.as_ref())? {
                tokio::time::sleep(Duration::from_secs(1)).await;
            } else {
                break
            }
        }

        let module_se = pipe::OpenOptions::new().open_sender(&module_path)?;
        let instruct_se = pipe::OpenOptions::new().open_sender(&instruct_path)?;
        let manipulate_se = pipe::OpenOptions::new().open_sender(&manipulate_path)?;

        loop {
            module_se.writable().await?;

            let req = ModuleInfoReq {
                name: "test".to_string(),
                client_type: ClientType::PipeType.into(),
                addr: vec![client_instruct_path.to_string(), client_manipulate_path.to_string()]
            };
            let mut data = vec![0; 1024];
            req.encode(&mut data)?;

            match module_se.try_write(&data) {
                Ok(_) => {
                    tracing::info!("module info send success!");
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
        loop {
            instruct_se.writable().await.unwrap();

            let req = InstructReq {
                instruct_type: InstructType::DefaultType.into(),
                message: vec!["ce4".to_string(), "shi".to_string()],
            };
            let mut data = vec![0; 1024];
            req.encode(&mut data)?;

            match instruct_se.try_write(&data) {
                Ok(_) => {
                    tracing::info!("instruct send success!");
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
        loop {
            manipulate_se.writable().await.unwrap();

            let req = ManipulateReq {
                manipulate_type: ManipulateType::DefaultType.into(),
                command: "test".to_string(),
            };
            let mut data = vec![0; 1024];
            req.encode(&mut data)?;

            match manipulate_se.try_write(&data) {
                Ok(_) => {
                    tracing::info!("instruct send success!");
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    Ok(())
}

async fn grpc_server(grpc_config: &GrpcConfig) -> Result<(), Box<dyn Error>> {
    if grpc_config.enable {
        let grpc_addr = format!("{}:{}", &grpc_config.addr, &grpc_config.port);
        tracing::debug!("server start!");
        Server::builder()
            .add_service(ManipulateServer::new(ManipulateImpl::default()))
            .add_service(InstructServer::new(InstructImpl::default()))
            .serve(grpc_addr.parse().unwrap()).await.unwrap();
    }
    Ok(())
}
