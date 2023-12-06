use std::collections::HashMap;

use anyhow::{anyhow, Result};
use nihility_common::response_code::RespCode;
use nihility_common::submodule::{ReceiveType, SubmoduleReq, SubmoduleType};
use tracing::debug;
use SubmoduleType::{GrpcType, HttpType, PipeType, WindowsNamedPipeType};

use crate::communicat::grpc;
#[cfg(unix)]
use crate::communicat::pipe::{PipeUnixInstructClient, PipeUnixManipulateClient};
#[cfg(windows)]
// use crate::communicat::windows_named_pipe::{
//     WindowsNamedPipeInstructClient, WindowsNamedPipeManipulateClient,
// };
use crate::communicat::{SendInstructOperate, SendManipulateOperate};
use crate::entity::instruct::TextInstructEntity;
use crate::entity::manipulate::SimpleManipulateEntity;

#[cfg(windows)]
// const INSTRUCT_WINDOWS_NAMED_PIPE_FIELD: &str = "instruct_windows_named_pipe";
#[cfg(windows)]
// const MANIPULATE_WINDOWS_NAMED_PIPE_FIELD: &str = "manipulate_windows_named_pipe";
#[cfg(unix)]
const INSTRUCT_PIPE_FIELD: &str = "instruct_pipe";
#[cfg(unix)]
const MANIPULATE_FIELD: &str = "manipulate_pipe";

/// 操作子模块类型
#[derive(Debug)]
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
#[derive(Debug)]
pub struct ModuleOperate {
    pub name: String,
    pub default_instruct: Vec<String>,
    pub submodule_type: SubmoduleType,
    pub receive_type: ReceiveType,
    pub conn_params: HashMap<String, String>,
    pub operate_type: OperateType,
}

/// 用于维护与子模块的连接以及处理对子模块的操作
pub struct Submodule {
    pub name: String,
    pub default_instruct_map: HashMap<String, String>,
    pub sub_module_type: SubmoduleType,
    pub receive_type: ReceiveType,
    pub heartbeat_time: u64,
    pub(crate) instruct_client: Box<dyn SendInstructOperate + Send + Sync>,
    pub(crate) manipulate_client: Box<dyn SendManipulateOperate + Send + Sync>,
}

impl ModuleOperate {
    /// 通过应用间消息创建操作子模块消息结构体，由调用的方法决定结构体类型
    pub fn create_by_req(req: SubmoduleReq, operate_type: OperateType) -> Self {
        ModuleOperate {
            name: req.name.clone(),
            default_instruct: req.default_instruct.clone(),
            submodule_type: req.clone().submodule_type(),
            receive_type: req.clone().receive_type(),
            conn_params: req.conn_params.clone(),
            operate_type,
        }
    }

    /// 通过已创建子模块构建子模块操作，无法获取连接地址，当此时已不需要这个变量
    ///
    /// 目前用来心跳过期时离线模块
    pub fn create_by_submodule(submodule: &Submodule, operate_type: OperateType) -> Self {
        let mut default_instruct = Vec::new();
        for (instruct, _) in submodule.default_instruct_map.iter() {
            default_instruct.push(instruct.to_string());
        }
        ModuleOperate {
            name: submodule.name.to_string(),
            default_instruct,
            submodule_type: submodule.sub_module_type,
            receive_type: submodule.receive_type,
            conn_params: HashMap::<String, String>::new(),
            operate_type,
        }
    }
}

impl Submodule {
    /// 统一实现由注册消息创建Module
    pub async fn create_by_operate(operate: ModuleOperate) -> Result<Self> {
        match operate.submodule_type {
            GrpcType => Ok(grpc::create_grpc_module(operate).await?),
            PipeType => {
                #[cfg(unix)]
                return Ok(Self::create_pipe_module(operate)?);
                #[cfg(windows)]
                Err(anyhow!("This OS Cannot Create PipeType Submodule"))
            }
            WindowsNamedPipeType => {
                #[cfg(unix)]
                return Err(anyhow!(
                    "This OS Cannot Create WindowsNamedPipeType Submodule"
                ));
                #[cfg(windows)]
                Ok(Self::create_windows_named_pipe_module(operate)?)
            }
            HttpType => Err(anyhow!("This Submodule Type Not Support Yet")),
        }
    }

    /// 创建pipe通信的子模块
    #[cfg(unix)]
    fn create_pipe_module(operate: ModuleOperate) -> Result<Submodule> {
        debug!("Start Create Pipe Submodule By {:?}", &operate);
        if let None = operate.conn_params.get(INSTRUCT_PIPE_FIELD) {
            return Err(anyhow!(
                "Create {:?} Type Submodule Error, ModuleOperate Missing {:?} Filed",
                &operate.submodule_type,
                INSTRUCT_PIPE_FIELD
            ));
        }
        if let None = operate.conn_params.get(MANIPULATE_PIPE_FIELD) {
            return Err(anyhow!(
                "Create {:?} Type Submodule Error, ModuleOperate Missing {:?} Filed",
                &operate.submodule_type,
                MANIPULATE_PIPE_FIELD
            ));
        }
        let instruct_path = operate.conn_params.get(INSTRUCT_PIPE_FIELD).unwrap();
        let manipulate_path = operate.conn_params.get(MANIPULATE_PIPE_FIELD).unwrap();
        let instruct_client = Box::new(PipeUnixInstructClient::init(instruct_path)?);
        let manipulate_client = Box::new(PipeUnixManipulateClient::init(manipulate_path)?);
        let mut instruct_map = HashMap::<String, String>::new();
        for instruct in operate.default_instruct {
            instruct_map.insert(instruct, String::new());
        }
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        debug!("Create Pipe Submodule {:?} Success", &operate.name);
        Ok(Submodule {
            name: operate.name,
            default_instruct_map: instruct_map,
            sub_module_type: operate.submodule_type,
            heartbeat_time: timestamp,
            instruct_client,
            manipulate_client,
        })
    }

    /// 创建WindowsNamedPipe通信的子模块
    #[cfg(windows)]
    fn create_windows_named_pipe_module(operate: ModuleOperate) -> Result<Submodule> {
        // debug!(
        //     "Start Create Windows Named Pipe Submodule By {:?}",
        //     &operate
        // );
        // if operate.conn_params.get(INSTRUCT_WINDOWS_NAMED_PIPE_FIELD).is_none() {
        //     return Err(anyhow!(
        //         "Create {:?} Type Submodule Error, ModuleOperate Missing {:?} Filed",
        //         &operate.submodule_type,
        //         INSTRUCT_WINDOWS_NAMED_PIPE_FIELD
        //     ));
        // }
        // if operate.conn_params.get(MANIPULATE_WINDOWS_NAMED_PIPE_FIELD).is_none() {
        //     return Err(anyhow!(
        //         "Create {:?} Type Submodule Error, ModuleOperate Missing {:?} Filed",
        //         &operate.submodule_type,
        //         MANIPULATE_WINDOWS_NAMED_PIPE_FIELD
        //     ));
        // }
        // let instruct_path = operate
        //     .conn_params
        //     .get(INSTRUCT_WINDOWS_NAMED_PIPE_FIELD)
        //     .unwrap();
        // let manipulate_path = operate
        //     .conn_params
        //     .get(MANIPULATE_WINDOWS_NAMED_PIPE_FIELD)
        //     .unwrap();
        // let instruct_client = Box::new(WindowsNamedPipeInstructClient::init(
        //     instruct_path.to_string(),
        // )?);
        // let manipulate_client = Box::new(WindowsNamedPipeManipulateClient::init(
        //     manipulate_path.to_string(),
        // )?);
        // let mut instruct_map = HashMap::<String, String>::new();
        // for instruct in operate.default_instruct {
        //     instruct_map.insert(instruct, String::new());
        // }
        // let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        // debug!(
        //     "Create Windows Named Pipe Submodule {:?} Success",
        //     &operate.name
        // );
        // Ok(Submodule {
        //     name: operate.name,
        //     default_instruct_map: instruct_map,
        //     sub_module_type: operate.submodule_type,
        //     receive_type: operate.receive_type,
        //     heartbeat_time: timestamp,
        //     instruct_client,
        //     manipulate_client,
        // })
        Err(anyhow!("unimplemented!!!"))
    }

    /// 模块发送指令由此方法统一执行
    pub async fn send_instruct(&mut self, instruct: TextInstructEntity) -> Result<RespCode> {
        debug!("Send Instruct Client Type: {:?}", self.sub_module_type);
        self.instruct_client.send(instruct.create_req()).await
    }

    /// 模块发送操作由此模块统一执行
    pub async fn send_manipulate(
        &mut self,
        manipulate: SimpleManipulateEntity,
    ) -> Result<RespCode> {
        debug!("Send Manipulate Client Type: {:?}", self.sub_module_type);
        self.manipulate_client.send(manipulate.create_req()).await
    }
}