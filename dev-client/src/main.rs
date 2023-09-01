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
    instruct::{
        instruct_client::InstructClient,
        InstructReq
    },
    manipulate::{
        manipulate_client::ManipulateClient,
        ManipulateReq
    }
};
use grpc::{ManipulateImpl, InstructImpl};
use nihility_common::instruct::instruct_server::InstructServer;
use nihility_common::instruct::InstructType;
use nihility_common::manipulate::manipulate_server::ManipulateServer;
use nihility_common::manipulate::ManipulateType;
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

async fn register(mut grpc_ip: String, grpc_addr: String) -> Result<(), tonic::Status> {
    tracing::info!("register start!");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    let in_addr = grpc_addr.clone();
    let man_addr = grpc_addr.clone();
    let mut module_info_client = ModuleInfoClient::connect(grpc_addr).await.unwrap();
    let mut instruct_client = InstructClient::connect(in_addr).await.unwrap();
    let mut manipulate_client = ManipulateClient::connect(man_addr).await.unwrap();

    let module_req = tonic::Request::new(ModuleInfoReq {
        name: "Tonic".into(),
        grpc_addr: grpc_ip.to_string(),
    });
    let message = vec!["fd".to_string(), "dsa".to_string()];
    let instruct_req = tonic::Request::new(InstructReq {
        instruct_type: InstructType::DefaultType.into(),
        message,
    });
    let manipulate_req = tonic::Request::new(ManipulateReq {
        manipulate_type: ManipulateType::DefaultType.into(),
        command: "test message".to_string(),
    });

    let module_info_resp = module_info_client.register(module_req).await.unwrap();
    tracing::info!("module_info_resp:{:?}", module_info_resp.into_inner());
    let instruct_resp = instruct_client.send_instruct(instruct_req).await.unwrap();
    tracing::info!("instruct_resp:{:?}", instruct_resp.into_inner());
    let manipulate_resp = manipulate_client.send_manipulate(manipulate_req).await.unwrap();
    tracing::info!("manipulate_resp:{:?}", manipulate_resp.into_inner());


    Ok(())
}

async fn grpc_server(mut grpc_ip: &mut String) -> () {
    tracing::info!("server start!");
    Server::builder()
        .add_service(ManipulateServer::new(ManipulateImpl::default()))
        .add_service(InstructServer::new(InstructImpl::default()))
        .serve(grpc_ip.parse().unwrap()).await.unwrap();
}
