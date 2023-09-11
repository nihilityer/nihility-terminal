use std::net::Ipv4Addr;
use std::str::FromStr;

use tokio::{
    io,
    sync::mpsc::{
        Receiver,
        Sender,
        error::TryRecvError
    },
    time::{
        self,
        Duration
    },
    net::{
        UdpSocket,
        unix::pipe,
    }
};
use tonic::transport::Server;

use nihility_common::{
    instruct::instruct_server::InstructServer,
    manipulate::manipulate_server::ManipulateServer,
    module_info::module_info_server::ModuleInfoServer,
};

use crate::alternate::{
    config::{GrpcConfig, ModuleManagerConfig, MulticastConfig, PipeConfig},
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

pub struct Multicaster {}

pub struct GrpcServer {}

pub struct PipeProcessor {}

pub struct ModuleManager {}

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
                tracing::debug!("发送组播信息：{}", multicast_config.multicast_info);
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
    pub async fn start(pipe_config: &PipeConfig) -> Result<(), AppError> {
        if pipe_config.enable {
            tracing::info!("PipeProcessor start!");
            let rx = pipe::OpenOptions::new().open_receiver(&pipe_config.unix.module)?;
            let tx = pipe::OpenOptions::new().open_sender(&pipe_config.unix.module)?;

            tracing::info!("FIFOProcessor start");
            loop {
                // Wait for the pipe to be writable
                tx.writable().await?;
                tracing::info!("start write");

                // Try to write data, this may still fail with `WouldBlock`
                // if the readiness event is a false positive.
                match tx.try_write(b"hello world") {
                    Ok(_) => {
                        break;
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            }
            loop {
                rx.readable().await?;
                let mut msg = vec![0; 1024];

                match rx.try_read(&mut msg) {
                    Ok(n) => {
                        let result = String::from_utf8(Vec::from(&msg[..n])).unwrap();
                        tracing::info!("{}", result);
                        break;
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
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
}

impl ModuleManager {
    pub async fn start(module_manager_config: &ModuleManagerConfig,
                       mut module_receiver: Receiver<Module>,
                       mut instruct_receiver: Receiver<InstructEntity>,
                       mut manipulate_receiver: Receiver<ManipulateEntity>, ) -> Result<(), AppError> {
        tracing::info!("ModuleManager start!");
        let mut module_list = Vec::new();
        loop {
            time::sleep(Duration::from_secs(module_manager_config.interval.into())).await;
            match module_receiver.try_recv() {
                Ok(module_value) => {
                    tracing::info!("注册module：{:?}", &module_value);
                    let _ = &module_list.push(module_value);
                }
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Empty => {}
                        TryRecvError::Disconnected => {
                            tracing::error!("模块注册Grpc服务异常！");
                            return Err(AppError::ModuleManagerError("模块注册".to_string()));
                        }
                    }
                }
            }
            match instruct_receiver.try_recv() {
                Ok(instruct_value) => {
                    tracing::info!("接收指令：{:?}", &instruct_value);
                    // TODO
                    for message in instruct_value.message {
                        for module in &module_list {
                            tracing::debug!("Module:{},message:{}", module.name, message);
                        }
                    }
                }
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Empty => {}
                        TryRecvError::Disconnected => {
                            tracing::error!("指令接收Grpc服务异常！");
                            return Err(AppError::ModuleManagerError("指令接收".to_string()));
                        }
                    }
                }
            }
            match manipulate_receiver.try_recv() {
                Ok(manipulate_value) => {
                    tracing::info!("接收到操作：{:?}", manipulate_value);
                    // TODO
                }
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Empty => {}
                        TryRecvError::Disconnected => {
                            tracing::error!("操作消息接收Grpc服务异常！");
                            return Err(AppError::ModuleManagerError("操作消息接收".to_string()));
                        }
                    }
                }
            }
        }
    }
}
