use std::error::Error;
use std::net::Ipv4Addr;

use tokio::net::UdpSocket;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::mpsc::error::TryRecvError;
use tokio::time;
use tokio::time::Duration;
use tonic::transport::{Server, server::Router};

use nihility_common::{
    instruct::instruct_server::InstructServer,
    manipulate::manipulate_server::ManipulateServer,
    module_info::module_info_server::ModuleInfoServer,
};

use crate::alternate::{
    config::NetConfig,
    implement::{
        InstructImpl,
        ManipulateImpl,
        ModuleInfoImpl,
    },
    module::{
        InstructEntity,
        ManipulateEntity,
        Module,
    }
};

// 发送组播播消息的间隔时间，单位：秒
const MULTICAST_INTERVAL:u64 = 10;
const MANAGER_INTERVAL:u64 = 2;

pub struct Multicaster {
    udp_socket: UdpSocket,
    grpc_addr: String,
    multicaster_addr: String,
}

pub struct GrpcServer {
    grpc_server_router: Router,
    grpc_server_addr: String,
}

pub struct ModuleManager {
    module_list: Vec<Module>,
    module_receiver: Receiver<Module>,
    instruct_receiver: Receiver<InstructEntity>,
    manipulate_receiver: Receiver<ManipulateEntity>,
}

impl Multicaster {

    pub async fn init(net_cfg: &NetConfig) -> Result<Self, Box<dyn Error>> {
        tracing::debug!("初始化udp_socket在：{}", &net_cfg.udp_addr);
        let udp_socket = UdpSocket::bind(&net_cfg.udp_addr).await?;
        udp_socket.join_multicast_v4(Ipv4Addr::new(224,0,0,123), Ipv4Addr::new(0,0,0,0))?;

        Ok(Multicaster {
            udp_socket,
            grpc_addr: net_cfg.grpc_addr.to_string(),
            multicaster_addr: net_cfg.multicast_port.to_string(),
        })
    }

    pub async fn start(self) -> Result<(), Box<dyn Error>> {
        tracing::debug!("Broadcaster start!");
        loop {
            let len = self.udp_socket.send_to(self.grpc_addr.as_bytes(), &*self.multicaster_addr).await?;
            tracing::debug!("向{}发送组播信息：{}, len: {}", self.multicaster_addr, self.grpc_addr, len);
            time::sleep(Duration::from_secs(MULTICAST_INTERVAL)).await;
        }
    }

}

impl GrpcServer {
    pub fn init(net_cfg: &NetConfig,
                module_sender: Sender<Module>,
                instruct_sender: Sender<InstructEntity>,
                manipulate_sender: Sender<ManipulateEntity>,
    ) -> Result<Self, Box<dyn Error>> {
        let router = Server::builder()
            .add_service(ModuleInfoServer::new(ModuleInfoImpl::init(module_sender)?))
            .add_service(InstructServer::new(InstructImpl::init(instruct_sender)?))
            .add_service(ManipulateServer::new(ManipulateImpl::init(manipulate_sender)?));

        Ok(GrpcServer {
            grpc_server_router: router,
            grpc_server_addr: net_cfg.grpc_addr.to_string(),
        })
    }

    pub async fn start(self) -> Result<(), Box<dyn Error>> {
        tracing::debug!("GrpcServer start!");
        self.grpc_server_router
            .serve(self.grpc_server_addr.parse().unwrap()).await?;
        Ok(())
    }
}

impl ModuleManager {
    pub fn init(module_receiver: Receiver<Module>,
                instruct_receiver: Receiver<InstructEntity>,
                manipulate_receiver: Receiver<ManipulateEntity>
    ) -> Result<Self, Box<dyn Error>> {
        Ok(ModuleManager {
            module_list: Vec::new(),
            module_receiver,
            instruct_receiver,
            manipulate_receiver,
        })
    }

    pub async fn start(mut self) -> Result<(), Box<dyn Error>> {
        tracing::info!("ModuleManager start!");
        loop {
            time::sleep(Duration::from_secs(MANAGER_INTERVAL)).await;
            match self.module_receiver.try_recv() {
                Ok(module_value) => {
                    tracing::info!("注册module：{:?}", &module_value);
                    let _ = &self.module_list.push(module_value);
                },
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Empty => {}
                        TryRecvError::Disconnected => {
                            tracing::error!("模块注册Grpc服务异常！");
                            return Err(Box::try_from(TryRecvError::Disconnected).unwrap())
                        }
                    }
                },
            }
            match self.instruct_receiver.try_recv() {
                Ok(instruct_value) => {
                    tracing::info!("接收指令：{:?}", &instruct_value);
                    // TODO
                    for message in instruct_value.message {
                        for module in &self.module_list {
                            tracing::debug!("Module:{},message:{}", module.name, message);
                        }
                    }
                }
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Empty => {}
                        TryRecvError::Disconnected => {
                            tracing::error!("指令接收Grpc服务异常！");
                            return Err(Box::try_from(TryRecvError::Disconnected).unwrap())
                        }
                    }
                }
            }
            match self.manipulate_receiver.try_recv() {
                Ok(manipulate_value) => {
                    tracing::info!("接收到操作：{:?}", manipulate_value);
                    // TODO
                }
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Empty => {}
                        TryRecvError::Disconnected => {
                            tracing::error!("操作消息接收Grpc服务异常！");
                            return Err(Box::try_from(TryRecvError::Disconnected).unwrap())
                        }
                    }
                }
            }
        }
    }
}
