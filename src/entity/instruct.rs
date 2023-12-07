use anyhow::{anyhow, Result};
use nihility_common::instruct::{InstructInfo, InstructType, TextInstruct};
use tracing::error;

use crate::entity::instruct::InstructData::Text;

#[derive(Debug)]
pub struct InstructInfoEntity {
    pub instruct_type: InstructType,
    pub receive_manipulate_submodule: String,
}

#[derive(Debug, Clone)]
pub enum InstructData {
    Text(String),
    Image,
}

/// 核心心模块内部传递的指令实体
#[derive(Debug)]
pub struct InstructEntity {
    pub info: Option<InstructInfoEntity>,
    pub instruct: InstructData,
}

impl InstructEntity {
    /// 通过外部请求实体创建内部指令实体
    pub fn create_by_req(req: TextInstruct) -> Self {
        if let Some(info) = req.info {
            InstructEntity {
                info: Some(InstructInfoEntity {
                    instruct_type: info.instruct_type(),
                    receive_manipulate_submodule: info.receive_manipulate_submodule,
                }),
                instruct: Text(req.instruct),
            }
        } else {
            InstructEntity {
                info: None,
                instruct: Text(req.instruct),
            }
        }
    }

    /// 由指令创建请求实体用于发送
    pub fn create_text_type_req(self) -> Result<TextInstruct> {
        if let Text(instruct) = self.instruct {
            if let Some(info) = self.info {
                Ok(TextInstruct {
                    info: Some(InstructInfo {
                        instruct_type: info.instruct_type.into(),
                        receive_manipulate_submodule: info.receive_manipulate_submodule,
                    }),
                    instruct,
                })
            } else {
                Ok(TextInstruct {
                    info: None,
                    instruct,
                })
            }
        } else {
            error!(
                "This Instruct Entity Cannot Create TextInstruct: {:?}",
                self
            );
            Err(anyhow!(
                "This Instruct Entity Cannot Create TextInstruct: {:?}",
                self
            ))
        }
    }
}
