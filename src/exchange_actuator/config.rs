pub struct NetConfig {
    // 用于初始化udp_socket的地址，反正不会用于接收消息
    pub udp_address: String,
    pub grpc_addr: String,
    pub grpc_port: String,
}

impl Default for NetConfig {
    fn default() -> Self {
        NetConfig {
            udp_address: "127.0.0.1:0".to_string(),
            grpc_addr: "127.0.0.1".to_string(),
            grpc_port: "5678".to_string(),
        }
    }
}