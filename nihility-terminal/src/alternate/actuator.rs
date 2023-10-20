use std::{
    fs,
    net::Ipv4Addr,
    path::Path,
    process::Command,
    str::FromStr,
};

use prost::Message;
use tokio::{
    io,
    net::{
        UdpSocket,
        unix::pipe,
    },
    sync::mpsc::Sender,
    time::{
        self,
        Duration,
    },
};
use tokio::sync::mpsc::error::TrySendError;
use tonic::transport::Server;

use nihility_common::{
    instruct::{
        instruct_server::InstructServer,
        InstructReq,
    },
    manipulate::{
        manipulate_server::ManipulateServer,
        ManipulateReq,
    },
    module_info::{
        module_info_server::ModuleInfoServer,
        ModuleInfoReq,
    },
};

use crate::alternate::{
    config::{GrpcConfig, MulticastConfig, PipeConfig},
    implement::{
        InstructImpl,
        ManipulateImpl,
        ModuleInfoImpl,
    },
    module::{
        InstructEntity,
        ManipulateEntity,
        Module,
    },
};
use crate::error::AppError;

pub struct Multicaster;

pub struct GrpcServer;

pub struct PipeProcessor;

impl Multicaster {
    pub async fn start(multicast_config: &MulticastConfig) -> Result<(), AppError> {
        if multicast_config.enable {
            tracing::debug!("Broadcaster start!");
            let mut bind_addr = multicast_config.bind_addr.to_string();
            bind_addr.push_str(format!(":{}", multicast_config.bind_port).as_str());
            tracing::debug!("初始化udp_socket在：{}", &bind_addr);
            let udp_socket = UdpSocket::bind(bind_addr).await?;

            let group_addr = Ipv4Addr::from_str(multicast_config.multicast_group.as_str())?;
            let interface_addr = Ipv4Addr::from_str(multicast_config.bind_addr.as_str())?;
            udp_socket.join_multicast_v4(group_addr, interface_addr)?;

            let mut multicast_addr = multicast_config.multicast_group.to_string();
            multicast_addr.push_str(format!(":{}", multicast_config.multicast_port).as_str());

            loop {
                tracing::debug!("向{}发送组播信息：{}", &multicast_addr, multicast_config.multicast_info);
                udp_socket.send_to(multicast_config.multicast_info.as_bytes(), multicast_addr.as_str()).await?;
                time::sleep(Duration::from_secs(multicast_config.interval.into())).await;
            }
        }
        Ok(())
    }
}

impl GrpcServer {
    pub async fn start(grpc_config: &GrpcConfig,
                       module_sender: Sender<Module>,
                       instruct_sender: Sender<InstructEntity>,
                       manipulate_sender: Sender<ManipulateEntity>, ) -> Result<(), AppError> {
        if grpc_config.enable {
            tracing::debug!("GrpcServer start!");
            let mut grpc_addr = grpc_config.addr.to_string();
            grpc_addr.push_str(format!(":{}", grpc_config.port).as_str());

            Server::builder()
                .add_service(ModuleInfoServer::new(ModuleInfoImpl::init(module_sender)))
                .add_service(InstructServer::new(InstructImpl::init(instruct_sender)))
                .add_service(ManipulateServer::new(ManipulateImpl::init(manipulate_sender)))
                .serve(grpc_addr.parse()?).await?;
        }

        Ok(())
    }
}

impl PipeProcessor {
    pub async fn start(pipe_config: &PipeConfig,
                       module_sender: Sender<Module>,
                       instruct_sender: Sender<InstructEntity>,
                       manipulate_sender: Sender<ManipulateEntity>) -> Result<(), AppError> {
        if pipe_config.enable {
            tracing::debug!("PipeProcessor start!");
            if cfg!(unix) {
                Self::unix_pipe_processor(&pipe_config, module_sender, instruct_sender, manipulate_sender).await?;
            } else if cfg!(windows) {
                // TODO windows平台支持代码
            } else {
                return Err(AppError::OsNotSupportError);
            }
        }
        Ok(())
    }

    /*
    pipe通信模块
     */
    async fn unix_pipe_processor(pipe_config: &PipeConfig,
                                 module_sender: Sender<Module>,
                                 instruct_sender: Sender<InstructEntity>,
                                 manipulate_sender: Sender<ManipulateEntity>) -> Result<(), AppError> {
        if !Path::try_exists(&pipe_config.unix.directory.as_ref())? {
            fs::create_dir_all(&pipe_config.unix.directory)?;
        }
        tracing::debug!("pipe work dir: {}", &pipe_config.unix.directory);

        let module_path = format!("{}/{}", pipe_config.unix.directory.as_str(), pipe_config.unix.module.to_string());
        let instruct_path = format!("{}/{}", pipe_config.unix.directory.as_str(), pipe_config.unix.instruct_receiver.to_string());
        let manipulate_path = format!("{}/{}", pipe_config.unix.directory.as_str(), pipe_config.unix.manipulate_receiver.to_string());

        if !Path::try_exists(&module_path.as_ref())? {
            tracing::debug!("cannot found module pipe file, try create on {}", &module_path);
            Command::new("mkfifo").arg(&module_path).output()?;
        }
        if !Path::try_exists(&instruct_path.as_ref())? {
            tracing::debug!("cannot found instruct pipe receiver file, try create on {}", &instruct_path);
            Command::new("mkfifo").arg(&instruct_path).output()?;
        }
        if !Path::try_exists(&manipulate_path.as_ref())? {
            tracing::debug!("cannot found manipulate pipe receiver file, try create on {}", &manipulate_path);
            Command::new("mkfifo").arg(&manipulate_path).output()?;
        }

        tracing::debug!("start create pipe from file");
        let module_rx = pipe::OpenOptions::new().open_receiver(&module_path)?;
        tracing::debug!("module pipe create success");
        let instruct_rx = pipe::OpenOptions::new().open_receiver(&instruct_path)?;
        tracing::debug!("instruct pipe create success");
        let manipulate_rx = pipe::OpenOptions::new().open_receiver(&manipulate_path)?;
        tracing::debug!("manipulate pipe create success");

        loop {
            Self::module_pipe_processor(&module_sender, &module_rx).await?;
            Self::instruct_pipe_processor(&instruct_sender, &instruct_rx).await?;
            Self::manipulate_pipe_processor(&manipulate_sender, &manipulate_rx).await?;
        }
    }

    /*
    负责接收pipe模块注册信息
     */
    async fn module_pipe_processor(module_sender: &Sender<Module>, module_rx: &pipe::Receiver) -> Result<(), AppError> {
        module_rx.readable().await?;
        let mut msg = vec![0; 1024];

        match module_rx.try_read(&mut msg) {
            Ok(n) => {
                if n > 0 {
                    let result: ModuleInfoReq = ModuleInfoReq::decode(&msg[..n])?;
                    tracing::debug!("pipe module name:{:?}", &result.name);
                    let model = Module::create_by_req(result).await?;
                    module_sender.send(model).await?;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(_) => {
                return Err(AppError::PipeError("module".to_string()));
            }
        }
        Ok(())
    }

    /*
    负责接收pipe指令信息
     */
    async fn instruct_pipe_processor(instruct_sender: &Sender<InstructEntity>, instruct_rx: &pipe::Receiver) -> Result<(), AppError> {
        instruct_rx.readable().await?;
        let mut data = vec![0; 1024];

        match instruct_rx.try_read(&mut data) {
            Ok(n) => {
                if n > 0 {
                    let result: InstructReq = InstructReq::decode(&data[..n])?;
                    let instruct_entity = InstructEntity::create_by_req(result);
                    tracing::debug!("pipe instruct: {:?}", &instruct_entity);
                    instruct_sender.send(instruct_entity).await?;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(_) => {
                return Err(AppError::PipeError("module".to_string()));
            }
        }
        Ok(())
    }

    /*
    负责接收pipe操作信息
     */
    async fn manipulate_pipe_processor(manipulate_sender: &Sender<ManipulateEntity>, manipulate_rx: &pipe::Receiver) -> Result<(), AppError> {
        manipulate_rx.readable().await?;
        let mut data = vec![0; 1024];

        match manipulate_rx.try_read(&mut data) {
            Ok(n) => {
                if n > 0 {
                    let result: ManipulateReq = ManipulateReq::decode(&data[..n])?;
                    let manipulate_entity = ManipulateEntity::create_by_req(result);
                    tracing::debug!("pipe manipulate: {:?}", &manipulate_entity);
                    manipulate_sender.send(manipulate_entity).await?;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(_) => {
                return Err(AppError::PipeError("module".to_string()));
            }
        }
        Ok(())
    }
}
