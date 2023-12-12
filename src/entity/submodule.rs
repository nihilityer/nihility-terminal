use std::collections::HashMap;

use anyhow::Result;
use nihility_common::response_code::RespCode;
use nihility_common::submodule::{ReceiveType, SubmoduleReq, SubmoduleType};
use tracing::debug;

use crate::communicat::{SendInstructOperate, SendManipulateOperate};
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::{ManipulateData, ManipulateEntity};

/// 操作子模块类型
#[derive(Debug)]
pub enum OperateType {
    /// 注册当前模块
    Register,
    /// 注销当前模块
    Offline,
    /// 当前模块心跳信息
    Heartbeat,
    /// 更新当前模块
    Update,
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
    /// 模块发送指令由此方法统一执行
    pub async fn send_instruct(&mut self, instruct: InstructEntity) -> Result<RespCode> {
        debug!("Send Instruct Client Type: {:?}", self.sub_module_type);
        self.instruct_client
            .send_text(instruct.create_text_type_req()?)
            .await
    }

    /// 模块发送操作由此模块统一执行
    pub async fn send_manipulate(
        &mut self,
        manipulate: ManipulateEntity,
    ) -> Result<RespCode> {
        debug!("Send Manipulate Client Type: {:?}", self.sub_module_type);
        match manipulate.manipulate {
            ManipulateData::Text(_) => {
                debug!("{:?} Client Send Text Type Manipulate", self.sub_module_type);
                self.manipulate_client.send_text_display(manipulate.create_text_type_req()?).await
            }
            ManipulateData::None => {
                debug!("{:?} Client Send Simple Type Manipulate", self.sub_module_type);
                self.manipulate_client.send_simple(manipulate.create_simple_type_req()?).await
            }
        }
    }
}
