use std::error::Error;
use tokio::net::UdpSocket;
use tokio::sync::oneshot::{Receiver, error::TryRecvError};
use crate::NetConfig;

pub struct Broadcaster {
    udp_socket: UdpSocket,
    grpc_addr: String,
    grpc_port: String,
    rx: Receiver<String>,
}

impl Broadcaster {

    pub async fn new(net_cfg: NetConfig, rx: Receiver<String>) -> Result<Self, Box<dyn Error>> {
        let udp_socket = UdpSocket::bind(net_cfg.udp_address).await?;
        if udp_socket.broadcast()? == false {
            udp_socket.set_broadcast(true)?
        }

        Ok(Broadcaster{
            udp_socket,
            grpc_addr: net_cfg.grpc_addr,
            grpc_port: net_cfg.grpc_port,
            rx
        })
    }

    pub async fn start(mut self) -> Result<(), Box<dyn Error>> {
        tracing::info!("Broadcaster start!");
        loop {
            match self.rx.try_recv() {
                Err(TryRecvError::Empty) => {
                    // TODO
                    self.udp_socket.send(&[1]);
                },
                Ok(_) => {
                    tracing::info!("Broadcaster exit!");
                    break
                },
                Err(TryRecvError::Closed) => {
                    tracing::error!("程序出现未知异常！");
                }
            }
        }
        Ok(())
    }

}