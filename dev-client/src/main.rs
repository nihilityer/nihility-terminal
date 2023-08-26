mod config;

extern crate nihility_common;

use nihility_common::submodule;

use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind("0.0.0.0:1234").await?;
    socket.set_broadcast(true)?;
    let mut buf = [0u8; 1024];

    println!("开始接收信息！");
    socket.recv_from(&mut buf).await?;
    let result = String::from_utf8(buf.to_vec()).unwrap();
    println!("{}", result);
    Ok(())
}
