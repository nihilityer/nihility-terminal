use local_ip_address::local_ip;
use std::error::Error;

const MULTICAST_ADDR:&str = "224.0.0.123:1234";

pub struct NetConfig {
    // 用于初始化udp_socket的地址，反正不会用于接收消息
    pub udp_addr: String,
    pub grpc_addr: String,
    pub multicast_port: String,
}

impl NetConfig {
    pub fn new(udp_port: String, grpc_port: String) -> Result<Self, Box<dyn Error>> {
        let mut udp_ip = local_ip()?.to_string();
        let mut grpc_ip = udp_ip.clone();

        udp_ip.push(':');
        udp_ip.push_str(&*udp_port);
        grpc_ip.push(':');
        grpc_ip.push_str(&*grpc_port);

        Ok(NetConfig{
            udp_addr: udp_ip,
            grpc_addr: grpc_ip,
            multicast_port: MULTICAST_ADDR.to_string(),
        })
    }

    pub fn default() -> Result<Self, Box<dyn Error>> {
        NetConfig::new(
            "0".to_string(),
            "5050".to_string(),
        )
    }
}
