mod grpc;

extern crate nihility_common;

use std::future::Future;
use std::net::Ipv4Addr;
use tokio::net::UdpSocket;
use tonic::transport::{Error, Server};
use local_ip_address::local_ip;
use tracing::Level;

use nihility_common::{
    module_info::{
        module_info_client::ModuleInfoClient,
        ModuleInfoReq,
    },
};
use grpc::{ManipulateImpl, InstructImpl};
use nihility_common::instruct::instruct_server::InstructServer;
use nihility_common::manipulate::manipulate_server::ManipulateServer;
use nihility_common::module_info::ModuleInfoResp;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_max_level(Level::DEBUG)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    let mut grpc_ip = local_ip()?.to_string();

    grpc_ip.push_str(":5051");

    let socket = UdpSocket::bind("0.0.0.0:1234").await?;
    socket.join_multicast_v4(Ipv4Addr::new(224,0,0,123), Ipv4Addr::new(0,0,0,0))?;
    let mut buf = [0u8; 1024];

    tracing::info!("开始接收信息！");
    let (count, _) = socket.recv_from(&mut buf).await?;
    tracing::info!("count:{}", count);
    let result = String::from_utf8(Vec::from(&buf[..count])).unwrap();
    let mut grpc_addr = "http://".to_string();
    grpc_addr.push_str(&result);
    tracing::info!("{}", grpc_addr);

    let clone_addr = grpc_ip.clone();
    let server = grpc_server(&mut grpc_ip);


    let reg = register(clone_addr, grpc_addr);

    tokio::join!(server, reg);

    Ok(())
}

async fn register(mut grpc_ip: String, mut grpc_addr: String) -> Result<tonic::Response<ModuleInfoResp>, tonic::Status> {
    tracing::info!("register start!");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    let mut module_info_client = ModuleInfoClient::connect(grpc_addr).await.unwrap();

    let request = tonic::Request::new(ModuleInfoReq {
        name: "Tonic".into(),
        grpc_addr: grpc_ip.to_string(),
    });

    tracing::info!("start send::{}!", &grpc_ip);
    let response = module_info_client.register(request).await.unwrap();


    Ok(response)
}

async fn grpc_server(mut grpc_ip: &mut String) -> () {
    tracing::info!("server start!");
    Server::builder()
        .add_service(ManipulateServer::new(ManipulateImpl::default()))
        .add_service(InstructServer::new(InstructImpl::default()))
        .serve(grpc_ip.parse().unwrap()).await.unwrap();
}
