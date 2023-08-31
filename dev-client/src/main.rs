mod grpc;

extern crate nihility_common;

use std::net::Ipv4Addr;
use tokio::net::UdpSocket;
use tonic::transport::Server;
use local_ip_address::local_ip;

use nihility_common::{
    module_info::{
        module_info_client::ModuleInfoClient,
        ModuleInfoReq,
    },
};
use grpc::{ManipulateImpl, InstructImpl};
use nihility_common::instruct::instruct_server::InstructServer;
use nihility_common::manipulate::manipulate_server::ManipulateServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut grpc_ip = local_ip()?.to_string()

    grpc_ip.push_str(":5051");

    let socket = UdpSocket::bind("0.0.0.0:1234").await?;
    socket.join_multicast_v4(Ipv4Addr::new(224,0,0,123), Ipv4Addr::new(0,0,0,0))?;
    let mut buf = [0u8; 1024];

    println!("开始接收信息！");
    let (count, _) = socket.recv_from(&mut buf).await?;
    println!("count:{}", count);
    let result = String::from_utf8(Vec::from(&buf[..count])).unwrap();
    let mut grpc_addr = "http://".to_string();
    grpc_addr.push_str(&result);
    println!("{}", grpc_addr);

    let mut module_info_client = ModuleInfoClient::connect(grpc_addr).await?;
    //
    let server = Server::builder()
        .add_service(ManipulateServer::new(ManipulateImpl::default()))
        .add_service(InstructServer::new(InstructImpl::default()))
        .serve(grpc_ip.parse().unwrap());;

    let request = tonic::Request::new(ModuleInfoReq {
        name: "Tonic".into(),
        grpc_addr: grpc_ip,
    });

    let response = module_info_client.register(request).await?;


    println!("{:?}", response.into_inner());

    tokio::join!(server);

    Ok(())
}
