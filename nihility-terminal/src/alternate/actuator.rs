use std::{
    net::Ipv4Addr,
    path::Path,
    process::Command,
    str::FromStr,
    fs
};


use tokio::{
    io,
    sync::mpsc::{
        Receiver,
        Sender,
        error::TryRecvError
    },
    time::{
        self,
        Duration
    },
    net::{
        UdpSocket,
        unix::pipe,
    }
};
use tonic::transport::Server;
use prost::Message;

use nihility_common::{
    instruct::{
        instruct_server::InstructServer,
        InstructReq,
    },
    manipulate::{
        manipulate_server::ManipulateServer,
        ManipulateReq,
    },
    module_info::{
        module_info_server::ModuleInfoServer,
        ModuleInfoReq,
    },
};

use crate::alternate::{
    config::{GrpcConfig, MulticastConfig, PipeConfig, SummaryConfig},
    implement::{
        InstructImpl,
        ManipulateImpl,
        ModuleInfoImpl,
    },
    module::{
        InstructEntity,
        ManipulateEntity,
        Module,
    },
};
use crate::error::AppError;

pub struct Multicaster {}

pub struct GrpcServer {}

pub struct PipeProcessor {}

pub struct ModuleManager {}

impl Multicaster {
    pub async fn start(multicast_config: &MulticastConfig) -> Result<(), AppError> {
        if multicast_config.enable {
            tracing::debug!("Broadcaster start!");
            let mut bind_addr = multicast_config.bind_addr.to_string();
            bind_addr.push_str(format!(":{}", multicast_config.bind_port).as_str());
            tracing::debug!("初始化udp_socket在：{}", &bind_addr);
            let udp_socket = UdpSocket::bind(bind_addr).await?;

            let group_addr = Ipv4Addr::from_str(multicast_config.multicast_group.as_str())?;
            let interface_addr = Ipv4Addr::from_str(multicast_config.bind_addr.as_str())?;
            udp_socket.join_multicast_v4(group_addr, interface_addr)?;

            let mut multicast_addr = multicast_config.multicast_group.to_string();
            multicast_addr.push_str(format!(":{}", multicast_config.multicast_port).as_str());

            loop {
                tracing::debug!("向{}发送组播信息：{}", &multicast_addr, multicast_config.multicast_info);
                udp_socket.send_to(multicast_config.multicast_info.as_bytes(), multicast_addr.as_str()).await?;
                time::sleep(Duration::from_secs(multicast_config.interval.into())).await;
            }
        }
        Ok(())
    }
}

impl GrpcServer {
    pub async fn start(grpc_config: &GrpcConfig,
                       module_sender: Sender<Module>,
                       instruct_sender: Sender<InstructEntity>,
                       manipulate_sender: Sender<ManipulateEntity>, ) -> Result<(), AppError> {
        if grpc_config.enable {
            tracing::debug!("GrpcServer start!");
            let mut grpc_addr = grpc_config.addr.to_string();
            grpc_addr.push_str(format!(":{}", grpc_config.port).as_str());

            Server::builder()
                .add_service(ModuleInfoServer::new(ModuleInfoImpl::init(module_sender)))
                .add_service(InstructServer::new(InstructImpl::init(instruct_sender)))
                .add_service(ManipulateServer::new(ManipulateImpl::init(manipulate_sender)))
                .serve(grpc_addr.parse()?).await?;
        }

        Ok(())
    }
}

impl PipeProcessor {
    pub async fn start(pipe_config: &PipeConfig,
                       module_sender: Sender<Module>,
                       instruct_sender: Sender<InstructEntity>,
                       manipulate_sender: Sender<ManipulateEntity>) -> Result<(), AppError> {
        if pipe_config.enable {
            tracing::debug!("PipeProcessor start!");
            if cfg!(unix) {
                Self::unix_pipe_processor(&pipe_config, module_sender, instruct_sender, manipulate_sender).await?;
            } else if cfg!(windows) {
                // TODO windows平台支持代码
            } else {
                return Err(AppError::OsNotSupportError)
            }
        }
        Ok(())
    }

    async fn unix_pipe_processor(pipe_config: &PipeConfig,
                                 module_sender: Sender<Module>,
                                 instruct_sender: Sender<InstructEntity>,
                                 manipulate_sender: Sender<ManipulateEntity>) -> Result<(), AppError> {

        if !Path::try_exists(&pipe_config.unix.directory.as_ref())? {
            fs::create_dir_all(&pipe_config.unix.directory)?;
        }

        let module_path = format!("{}/{}", pipe_config.unix.directory.as_str(), pipe_config.unix.module.to_string());
        let instruct_path = format!("{}/{}", pipe_config.unix.directory.as_str(), pipe_config.unix.instruct_receiver.to_string());
        let manipulate_path = format!("{}/{}", pipe_config.unix.directory.as_str(), pipe_config.unix.manipulate_receiver.to_string());

        if !Path::try_exists(&module_path.as_ref())? {
            Command::new("mkfifo").arg(&module_path).spawn()?;
        }
        if !Path::try_exists(&instruct_path.as_ref())? {
            Command::new("mkfifo").arg(&instruct_path).spawn()?;
        }
        if !Path::try_exists(&manipulate_path.as_ref())? {
            Command::new("mkfifo").arg(&manipulate_path).spawn()?;
        }

        loop {
            if !Path::try_exists(&module_path.as_ref())? || !Path::try_exists(&instruct_path.as_ref())? || !Path::try_exists(&manipulate_path.as_ref())? {
                time::sleep(Duration::from_secs(1)).await;
            } else {
                break
            }
        }

        let module_rx = pipe::OpenOptions::new().open_receiver(&module_path)?;
        let instruct_rx = pipe::OpenOptions::new().open_receiver(&instruct_path)?;
        let manipulate_rx = pipe::OpenOptions::new().open_receiver(&manipulate_path)?;

        loop {
            Self::module_pipe_processor(&module_sender, &module_rx).await?;
            Self::instruct_pipe_processor(&instruct_sender, &instruct_rx).await?;
            Self::manipulate_pipe_processor(&manipulate_sender, &manipulate_rx).await?;
        }
    }

    async fn module_pipe_processor(module_sender: &Sender<Module>, module_rx: &pipe::Receiver) -> Result<(), AppError> {
        module_rx.readable().await?;
        let mut msg = vec![0; 1024];

        match module_rx.try_read(&mut msg) {
            Ok(n) => {
                let result: ModuleInfoReq = ModuleInfoReq::decode(&msg[..n])?;
                tracing::debug!("pipe module name:{}", &result.name);
                module_sender.send(Module::create_by_req(result).await?).await?;
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(_) => {
                return Err(AppError::PipeError("module".to_string()));
            }
        }
        Ok(())
    }

    async fn instruct_pipe_processor(instruct_sender: &Sender<InstructEntity>, instruct_rx: &pipe::Receiver) -> Result<(), AppError> {
        instruct_rx.readable().await?;
        let mut data = vec![0; 1024];

        match instruct_rx.try_read(&mut data) {
            Ok(n) => {
                let result: InstructReq = InstructReq::decode(&data[..n])?;
                let instruct_entity = InstructEntity::create_by_req(result);
                tracing::debug!("pipe instruct name:{:?}", &instruct_entity);
                instruct_sender.send(instruct_entity).await?;
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(_) => {
                return Err(AppError::PipeError("module".to_string()));
            }
        }
        Ok(())
    }

    async fn manipulate_pipe_processor(manipulate_sender: &Sender<ManipulateEntity>, manipulate_rx: &pipe::Receiver) -> Result<(), AppError> {
        manipulate_rx.readable().await?;
        let mut data = vec![0; 1024];

        match manipulate_rx.try_read(&mut data) {
            Ok(n) => {
                let result: ManipulateReq = ManipulateReq::decode(&data[..n])?;
                let instruct_entity = ManipulateEntity::create_by_req(result);
                tracing::debug!("pipe manipulate name:{:?}", &instruct_entity);
                manipulate_sender.send(instruct_entity).await?;
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(_) => {
                return Err(AppError::PipeError("module".to_string()));
            }
        }
        Ok(())
    }

}

impl ModuleManager {
    pub async fn start(summary_config: &SummaryConfig,
                       grpc_module_receiver: Receiver<Module>,
                       grpc_instruct_receiver: Receiver<InstructEntity>,
                       grpc_manipulate_receiver: Receiver<ManipulateEntity>,
                       pipe_module_receiver: Receiver<Module>,
                       pipe_instruct_receiver: Receiver<InstructEntity>,
                       pipe_manipulate_receiver: Receiver<ManipulateEntity>,
    ) -> Result<(), AppError> {
        tracing::debug!("ModuleManager start!");
        let mut module_list = Vec::new();
        let mut module_re_list = Vec::new();
        let mut instruct_re_list = Vec::new();
        let mut manipulate_re_list = Vec::new();

        if summary_config.grpc.enable {
            module_re_list.push(grpc_module_receiver);
            instruct_re_list.push(grpc_instruct_receiver);
            manipulate_re_list.push(grpc_manipulate_receiver);
        }
        if summary_config.pipe.enable {
            module_re_list.push(pipe_module_receiver);
            instruct_re_list.push(pipe_instruct_receiver);
            manipulate_re_list.push(pipe_manipulate_receiver);
        }

        loop {
            time::sleep(Duration::from_secs(summary_config.module_manager.interval.into())).await;

            Self::manager_module(&mut module_list, &mut module_re_list)?;

            Self::manager_instruct(&mut module_list, &mut instruct_re_list)?;

            Self::manager_manipulate(&mut module_list, &mut manipulate_re_list)?;
        }
    }

    fn manager_manipulate(module_list: &mut Vec<Module>, manipulate_re_list: &mut Vec<Receiver<ManipulateEntity>>) -> Result<(), AppError> {
        for manipulate_re in manipulate_re_list {
            match manipulate_re.try_recv() {
                Ok(manipulate_value) => {
                    tracing::info!("接收到操作：{:?}", manipulate_value);
                    // TODO
                    for (_, module) in module_list.iter_mut().enumerate() {
                        tracing::debug!("module name:{}", module.name)
                    }
                }
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Empty => {}
                        TryRecvError::Disconnected => {
                            tracing::error!("操作消息接收Grpc服务异常！");
                            return Err(AppError::ModuleManagerError("操作消息接收".to_string()));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn manager_instruct(module_list: &mut Vec<Module>, instruct_re_list: &mut Vec<Receiver<InstructEntity>>) -> Result<(), AppError> {
        for instruct_re in instruct_re_list {
            match instruct_re.try_recv() {
                Ok(instruct_value) => {
                    tracing::info!("接收指令：{:?}", &instruct_value);
                    // TODO
                    for message in instruct_value.message {
                        for module in module_list.iter_mut() {
                            tracing::debug!("Module:{},message:{}", module.name, message);
                        }
                    }
                }
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Empty => {}
                        TryRecvError::Disconnected => {
                            tracing::error!("指令接收Grpc服务异常！");
                            return Err(AppError::ModuleManagerError("指令接收".to_string()));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn manager_module(module_list: &mut Vec<Module>, module_re_list: &mut Vec<Receiver<Module>>) -> Result<(), AppError> {
        for module_re in module_re_list {
            match module_re.try_recv() {
                Ok(module_value) => {
                    tracing::info!("注册module：{:?}", &module_value);
                    let _ = &module_list.push(module_value);
                }
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Empty => {}
                        TryRecvError::Disconnected => {
                            tracing::error!("模块注册Grpc服务异常！");
                            return Err(AppError::ModuleManagerError("模块注册".to_string()));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
