use std::net::Ipv4Addr;
use std::str::FromStr;

use tokio::net::UdpSocket;
use tokio::time::{self, Duration};

use crate::AppError;
use crate::config::MulticastConfig;

pub struct Multicast;

impl Multicast {
    pub async fn start(multicast_config: &MulticastConfig) -> Result<(), AppError> {
        if multicast_config.enable {
            tracing::info!("Multicast start");
            let mut bind_addr = multicast_config.bind_addr.to_string();
            bind_addr.push_str(format!(":{}", multicast_config.bind_port).as_str());
            tracing::debug!("bind udp_socket on: {}", &bind_addr);
            let udp_socket = UdpSocket::bind(bind_addr).await?;

            let group_addr = Ipv4Addr::from_str(multicast_config.multicast_group.as_str())?;
            let interface_addr = Ipv4Addr::from_str(multicast_config.bind_addr.as_str())?;
            udp_socket.join_multicast_v4(group_addr, interface_addr)?;

            let mut multicast_addr = multicast_config.multicast_group.to_string();
            multicast_addr.push_str(format!(":{}", multicast_config.multicast_port).as_str());

            loop {
                tracing::debug!(
                    "towards {} send {}",
                    &multicast_addr,
                    multicast_config.multicast_info
                );
                udp_socket
                    .send_to(
                        multicast_config.multicast_info.as_bytes(),
                        multicast_addr.as_str(),
                    )
                    .await?;
                time::sleep(Duration::from_secs(multicast_config.interval.into())).await;
            }
        }
        Ok(())
    }
}
