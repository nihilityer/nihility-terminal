use std::error::Error;
use tokio::net::UdpSocket;
use tokio::time;
use tokio::time::Duration;
use tonic::transport::{server::Router, Server};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::alternate::{
    config::NetConfig,
    implement::{
        ModuleInfoImpl,
        InstructImpl,
        ManipulateImpl,
    },
    module::{
        Module,
        InstructEntity,
        ManipulateEntity,
    }
};

use nihility_common::{
    module_info::module_info_server::ModuleInfoServer,
    instruct::instruct_server::InstructServer,
    manipulate::manipulate_server::ManipulateServer,
};

// 发送广播消息的间隔时间，单位：秒
const BROADCAST_INTERVAL:u64 = 10;

pub struct Broadcaster {
    udp_socket: UdpSocket,
    grpc_addr: String,
    broadcast_addr: String,
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
    instruct_sender_list: Vec<Sender<InstructEntity>>,
    manipulate_sender_list: Vec<Sender<ManipulateEntity>>,
}

impl Broadcaster {

    pub async fn init(net_cfg: &NetConfig) -> Result<Self, Box<dyn Error>> {
        tracing::debug!("初始化udp_socket在：{}", &net_cfg.udp_addr);
        let udp_socket = UdpSocket::bind(&net_cfg.udp_addr).await?;
        if udp_socket.broadcast()? == false {
            udp_socket.set_broadcast(true)?
        }

        Ok(Broadcaster{
            udp_socket,
            grpc_addr: net_cfg.grpc_addr.to_string(),
            broadcast_addr: net_cfg.broadcast_port.to_string(),
        })
    }

    pub async fn start(self) -> Result<(), Box<dyn Error>> {
        tracing::debug!("Broadcaster start!");
        loop {
            let len = self.udp_socket.send_to(self.grpc_addr.as_bytes(), &*self.broadcast_addr).await?;
            tracing::debug!("向{}发送广播信息：{}, len: {}", self.broadcast_addr, self.grpc_addr, len);
            time::sleep(Duration::from_secs(BROADCAST_INTERVAL)).await;
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
            instruct_sender_list: Vec::new(),
            manipulate_sender_list: Vec::new(),
        })
    }
}
