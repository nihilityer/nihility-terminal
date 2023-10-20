use std::fmt::Debug;

use async_trait::async_trait;
use tonic::Request;
use tonic::transport::Channel;

use nihility_common::{
    instruct::{
        instruct_client::InstructClient,
        InstructReq,
        InstructType,
    },
    manipulate::{
        manipulate_client::ManipulateClient,
        ManipulateReq,
        ManipulateType,
    },
    module_info::{
        ClientType,
        ModuleInfoReq,
    },
};

use crate::alternate::pipe::{PipeUnixInstructClient, PipeUnixManipulateClient};
use crate::AppError;

/*
发送指令特征
 */
#[async_trait]
pub trait SendInstructOperate {
    /*
    发送指令
     */
    async fn send(&mut self, instruct: InstructReq) -> Result<bool, AppError>;
}

#[async_trait]
impl SendInstructOperate for InstructClient<Channel> {
    async fn send(&mut self, instruct: InstructReq) -> Result<bool, AppError> {
        let result = self.send_instruct(Request::new(instruct)).await?.into_inner();
        Ok(result.status)
    }
}

#[async_trait]
impl SendInstructOperate for PipeUnixInstructClient {
    async fn send(&mut self, instruct: InstructReq) -> Result<bool, AppError> {
        let result = self.send_instruct(instruct).await?;
        Ok(result.status)
    }
}

/*
发送操作特征
 */
#[async_trait]
pub trait SendManipulateOperate {
    /*
    发送操作
     */
    async fn send(&mut self, manipulate: ManipulateReq) -> Result<bool, AppError>;
}

#[async_trait]
impl SendManipulateOperate for ManipulateClient<Channel> {
    async fn send(&mut self, manipulate: ManipulateReq) -> Result<bool, AppError> {
        let result = self.send_manipulate(Request::new(manipulate)).await?.into_inner();
        Ok(result.status)
    }
}

#[async_trait]
impl SendManipulateOperate for PipeUnixManipulateClient {
    async fn send(&mut self, manipulate: ManipulateReq) -> Result<bool, AppError> {
        let result = self.send_manipulate(manipulate).await?;
        Ok(result.status)
    }
}

/*
用于维护与子模块的连接以及处理对子模块的操作
 */
pub struct Module {
    pub name: String,
    client_type: ClientType,
    instruct_client: Box<dyn SendInstructOperate + Send>,
    manipulate_client: Box<dyn SendManipulateOperate + Send>,
}

/*
核心心模块内部传递的指令实体
 */
#[derive(Debug)]
pub struct InstructEntity {
    pub instruct_type: InstructType,
    pub message: Vec<String>,
}

/*
核心模块内部传递的操作实体
 */
#[derive(Debug)]
pub struct ManipulateEntity {
    pub manipulate_type: ManipulateType,
    pub command: String,
}

impl Module {
    /*
    统一实现由注册消息创建Module
     */
    pub async fn create_by_req(req: ModuleInfoReq) -> Result<Self, AppError> {
        let client_type = match ClientType::from_i32(req.client_type) { 
            Some(result) => {
                tracing::debug!("{:?}", result);
                result
            },
            None => {
                return Err(AppError::ProstTransferError(String::from("module")))
            }
        };
        return match client_type {
            ClientType::GrpcType => {
                Ok(Self::create_grpc_module(req, client_type).await?)
            },
            ClientType::PipeType => {
                Ok(Self::create_pipe_module(req, client_type)?)
            }
        }
    }

    /*
    创建pipe通信的子模块
     */
    fn create_pipe_module(req: ModuleInfoReq, client_type: ClientType) -> Result<Module, AppError> {
        tracing::debug!("start create pipe module");
        let instruct_path = req.addr[0].to_string();
        let manipulate_path = req.addr[1].to_string();
        let instruct_client = Box::new(PipeUnixInstructClient::init(instruct_path)?);
        let manipulate_client = Box::new(PipeUnixManipulateClient::init(manipulate_path)?);
        tracing::debug!("create pipe module {} success", &req.name);
        Ok(Module {
            name: req.name,
            client_type,
            instruct_client,
            manipulate_client,
        })
    }

    /*
    创建grpc通信的子模块
     */
    async fn create_grpc_module(req: ModuleInfoReq, client_type: ClientType) -> Result<Module, AppError> {
        tracing::debug!("start create grpc module");
        let grpc_addr = format!("http://{}", req.addr[0]);
        let instruct_client: Box<InstructClient<Channel>> = Box::new(InstructClient::connect(grpc_addr.to_string()).await?);
        let manipulate_client: Box<ManipulateClient<Channel>> = Box::new(ManipulateClient::connect(grpc_addr.to_string()).await?);
        Ok(Module {
            name: req.name,
            client_type,
            instruct_client,
            manipulate_client,
        })
    }

    /*
    模块发送指令由此方法统一执行
     */
    pub async fn send_instruct(&mut self, instruct: InstructEntity) -> Result<bool, AppError> {
        tracing::debug!("send instruct client type:{:?}", self.client_type);
        let result = self.instruct_client.send(instruct.create_req()).await?;
        Ok(result)
    }

    /*
    模块发送操作由此模块统一执行
     */
    pub async fn send_manipulate(&mut self, manipulate: ManipulateEntity) -> Result<bool, AppError> {
        tracing::debug!("send manipulate client type:{:?}", self.client_type);
        let result = self.manipulate_client.send(manipulate.create_req()).await?;
        Ok(result)
    }
}

impl InstructEntity {
    /*
    通过外部请求实体创建内部指令实体
     */
    pub fn create_by_req(req: InstructReq) -> Self {
        let instruct_type = match InstructType::from_i32(req.instruct_type) {
            Some(result) => {
                result
            },
            None => {
                InstructType::DefaultType
            }
        };
        InstructEntity {
            instruct_type,
            message: req.message,
        }
    }

    /*
    由指令创建请求实体用于发送
     */
    pub fn create_req(self) -> InstructReq {
        InstructReq {
            instruct_type: self.instruct_type.into(),
            message: self.message,
        }
    }
}

impl ManipulateEntity {
    /*
    通过外部请求实体创建内部操作实体
     */
    pub fn create_by_req(req: ManipulateReq) -> Self {
        let manipulate_type = match ManipulateType::from_i32(req.manipulate_type) {
            Some(result) => {
                result
            },
            None => {
                ManipulateType::DefaultType
            }
        };
        ManipulateEntity {
            manipulate_type,
            command: req.command,
        }
    }

    /*
    有操作创建请求实体用于发送
     */
    pub fn create_req(self) -> ManipulateReq {
        ManipulateReq {
            manipulate_type: self.manipulate_type.into(),
            command: self.command,
        }
    }
}
