use anyhow::{anyhow, Result};
use nihility_common::instruct::{InstructInfo, InstructType, TextInstruct};

use crate::entity::instruct::InstructData::Text;

#[derive(Debug)]
pub struct InstructInfoEntity {
    pub instruct_type: InstructType,
    pub receive_manipulate_submodule: String,
}

#[derive(Debug, Clone)]
pub enum InstructData {
    Text(String),
}

/// 核心心模块内部传递的指令实体
#[derive(Debug)]
pub struct InstructEntity {
    pub info: Option<InstructInfoEntity>,
    pub instruct: InstructData,
}

impl InstructInfoEntity {
    pub fn create_by_req_info(req_info: Option<InstructInfo>) -> Option<Self> {
        match req_info {
            None => None,
            Some(info) => Some(InstructInfoEntity {
                instruct_type: info.instruct_type(),
                receive_manipulate_submodule: info.receive_manipulate_submodule,
            }),
        }
    }

    pub fn create_req_info(self) -> InstructInfo {
        InstructInfo {
            instruct_type: self.instruct_type.into(),
            receive_manipulate_submodule: self.receive_manipulate_submodule,
        }
    }
}

impl InstructEntity {
    pub fn create_by_text_type_req(req: TextInstruct) -> Self {
        InstructEntity {
            info: InstructInfoEntity::create_by_req_info(req.info),
            instruct: Text(req.instruct),
        }
    }

    pub fn create_text_type_req(self) -> Result<TextInstruct> {
        match self.instruct {
            Text(instruct) => match self.info {
                None => Ok(TextInstruct {
                    info: None,
                    instruct,
                }),
                Some(info) => Ok(TextInstruct {
                    info: Some(info.create_req_info()),
                    instruct,
                }),
            },
            data_type => Err(anyhow!(
                "This Manipulate Entity Is In Other Type, Please Create {:?} Type Req",
                data_type
            )),
        }
    }
}
