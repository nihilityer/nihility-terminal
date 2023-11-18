use std::borrow::Borrow;
use std::collections::HashMap;

use nihility_common::instruct::instruct_client::InstructClient;
use nihility_common::manipulate::manipulate_client::ManipulateClient;
use nihility_common::response_code::RespCode;
use nihility_common::sub_module::{ModuleInfo, SubModuleType};
use tonic::transport::Channel;

use crate::AppError;
use crate::communicat::{SendInstructOperate, SendManipulateOperate};
#[cfg(unix)]
use crate::communicat::pipe::{PipeUnixInstructClient, PipeUnixManipulateClient};
#[cfg(windows)]
use crate::communicat::windows_named_pipe::{
    WindowsNamedPipeInstructClient, WindowsNamedPipeManipulateClient,
};
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;

/// 操作子模块类型
pub enum OperateType {
    /// 注册当前模块
    REGISTER,
    /// 注销当前模块
    OFFLINE,
    /// 当前模块心跳信息
    HEARTBEAT,
    /// 更新当前模块
    UPDATE,
}

/// 操作子模块消息结构体
pub struct ModuleOperate {
    pub name: String,
    pub default_instruct: Vec<String>,
    pub sub_module_type: SubModuleType,
    pub addr: Vec<String>,
    pub operate_type: OperateType,
}

/// 用于维护与子模块的连接以及处理对子模块的操作
pub struct Module {
    pub name: String,
    pub default_instruct_map: HashMap<String, String>,
    pub sub_module_type: SubModuleType,
    instruct_client: Box<dyn SendInstructOperate + Send>,
    manipulate_client: Box<dyn SendManipulateOperate + Send>,
}

impl ModuleOperate {
    /// 通过应用间消息创建操作子模块消息结构体，由调用的方法决定结构体类型
    pub fn create_by_req(req: ModuleInfo, operate_type: OperateType) -> Result<Self, AppError> {
        if let Some(sub_module_type) = SubModuleType::from_i32(req.sub_module_type) {
            Ok(ModuleOperate {
                name: req.name,
                default_instruct: req.default_instruct,
                sub_module_type,
                addr: req.addr,
                operate_type,
            })
        } else {
            Err(AppError::ProstTransferError(String::from("model")))
        }
    }
}

impl Module {
    /// 统一实现由注册消息创建Module
    pub async fn create_by_operate(operate: ModuleOperate) -> Result<Self, AppError> {
        return match operate.sub_module_type {
            SubModuleType::GrpcType => Ok(Self::create_grpc_module(operate).await?),
            SubModuleType::PipeType => {
                #[cfg(unix)]
                return Ok(Self::create_pipe_module(operate)?);
                #[cfg(windows)]
                return Err(AppError::ModuleManagerError(
                    "not support model type".to_string(),
                ));
            }
            SubModuleType::WindowsNamedPipeType => {
                #[cfg(unix)]
                return Err(AppError::ModuleManagerError(
                    "not support model type".to_string(),
                ));
                #[cfg(windows)]
                return Ok(Self::create_windows_named_pipe_module(operate)?);
            }
        };
    }

    /// 创建pipe通信的子模块
    #[cfg(unix)]
    fn create_pipe_module(operate: ModuleOperate) -> Result<Module, AppError> {
        tracing::debug!("start create pipe model");
        let instruct_path = operate.addr[0].to_string();
        let manipulate_path = operate.addr[1].to_string();
        let instruct_client = Box::new(PipeUnixInstructClient::init(instruct_path)?);
        let manipulate_client = Box::new(PipeUnixManipulateClient::init(manipulate_path)?);
        let mut instruct_map = HashMap::<String, String>::new();
        for instruct in operate.default_instruct {
            instruct_map.insert(instruct, String::new());
        }
        tracing::debug!("create pipe model {} success", &req.name);
        Ok(Module {
            name: operate.name,
            default_instruct: operate.default_instruct.into(),
            instruct_points_id: Vec::<String>::new(),
            sub_module_type: operate.sub_module_type,
            instruct_client,
            manipulate_client,
        })
    }

    /// 创建WindowsNamedPipe通信的子模块
    #[cfg(windows)]
    fn create_windows_named_pipe_module(operate: ModuleOperate) -> Result<Module, AppError> {
        tracing::debug!("start create pipe model");
        let instruct_path = operate.addr[0].to_string();
        let manipulate_path = operate.addr[1].to_string();
        let instruct_client = Box::new(WindowsNamedPipeInstructClient::init(instruct_path)?);
        let manipulate_client = Box::new(WindowsNamedPipeManipulateClient::init(manipulate_path)?);
        let mut instruct_map = HashMap::<String, String>::new();
        for instruct in operate.default_instruct {
            instruct_map.insert(instruct, String::new());
        }
        tracing::debug!("create pipe model {} success", &operate.name);
        Ok(Module {
            name: operate.name,
            default_instruct_map: instruct_map,
            sub_module_type: operate.sub_module_type,
            instruct_client,
            manipulate_client,
        })
    }

    /// 创建grpc通信的子模块
    async fn create_grpc_module(operate: ModuleOperate) -> Result<Module, AppError> {
        tracing::debug!("start create grpc model");
        let grpc_addr = operate.addr[0].to_string();
        let instruct_client: Box<InstructClient<Channel>> =
            Box::new(InstructClient::connect(grpc_addr.to_string()).await?);
        let manipulate_client: Box<ManipulateClient<Channel>> =
            Box::new(ManipulateClient::connect(grpc_addr.to_string()).await?);
        let mut instruct_map = HashMap::<String, String>::new();
        for instruct in operate.default_instruct {
            instruct_map.insert(instruct, String::new());
        }
        tracing::debug!("create grpc model {} success", &operate.name);
        Ok(Module {
            name: operate.name,
            default_instruct_map: instruct_map,
            sub_module_type: operate.sub_module_type,
            instruct_client,
            manipulate_client,
        })
    }

    /// 模块发送指令由此方法统一执行
    pub async fn send_instruct(&mut self, instruct: InstructEntity) -> Result<bool, AppError> {
        tracing::debug!("send instruct client type:{:?}", self.sub_module_type);
        let result = self.instruct_client.send(instruct.create_req()).await?;
        if result.eq(RespCode::Success.borrow()) {
            return Ok(true);
        }
        tracing::debug!(
            "{:?} send_instruct error: {:?}",
            self.name,
            RespCode::from_i32(result.into())
        );
        Ok(false)
    }

    /// 模块发送操作由此模块统一执行
    pub async fn send_manipulate(
        &mut self,
        manipulate: ManipulateEntity,
    ) -> Result<bool, AppError> {
        tracing::debug!("send manipulate client type:{:?}", self.sub_module_type);
        let result = self.manipulate_client.send(manipulate.create_req()).await?;
        if result.eq(RespCode::Success.borrow()) {
            return Ok(true);
        }
        tracing::debug!(
            "{:?} send_manipulate error: {:?}",
            self.name,
            RespCode::from_i32(result.into())
        );
        Ok(false)
    }
}
