use std::error::Error;
use tokio::net::UdpSocket;
use tokio::time;
use tokio::time::Duration;
use tonic::transport::{server::Router, Server};

use crate::alternate::config::NetConfig;
use crate::alternate::achieve::{
    ModuleInfoImpl,
    InstructImpl,
    ManipulateImpl,
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

pub struct GrpcServer {
    grpc_server_router: Router,
    grpc_server_addr: String,
}

impl GrpcServer {
    pub fn init(net_cfg: &NetConfig) -> Result<Self, Box<dyn Error>> {
        let router = Server::builder()
            .add_service(ModuleInfoServer::new(ModuleInfoImpl::default()))
            .add_service(InstructServer::new(InstructImpl::default()))
            .add_service(ManipulateServer::new(ManipulateImpl::default()));

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