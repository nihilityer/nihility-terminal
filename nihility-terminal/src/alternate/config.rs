use local_ip_address::local_ip;
use std::error::Error;

pub struct NetConfig {
    // 用于初始化udp_socket的地址，反正不会用于接收消息
    pub udp_addr: String,
    pub grpc_addr: String,
    pub broadcast_port: String,
}

impl NetConfig {
    pub fn new(udp_port: String, grpc_port: String, broadcast_port: String) -> Result<Self, Box<dyn Error>> {
        let mut udp_ip = local_ip()?.to_string();
        let mut grpc_ip = udp_ip.clone();

        udp_ip.push(':');
        udp_ip.push_str(&*udp_port);
        grpc_ip.push(':');
        grpc_ip.push_str(&*grpc_port);

        let mut broadcast_addr = "253.255.255.255:".to_string();
        broadcast_addr.push_str(&*broadcast_port);

        Ok(NetConfig{
            udp_addr: udp_ip,
            grpc_addr: grpc_ip,
            broadcast_port: broadcast_addr,
        })
    }

    pub fn default() -> Result<Self, Box<dyn Error>> {
        NetConfig::new(
            "0".to_string(),
            "5050".to_string(),
            "1234".to_string(),
        )
    }
}
