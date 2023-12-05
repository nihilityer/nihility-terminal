use nihility_common::instruct::{InstructInfo, InstructType, TextInstruct};

#[derive(Debug)]
pub struct InstructInfoEntity {
    pub instruct_type: InstructType,
    pub receive_manipulate_submodule: String,
}

/// 核心心模块内部传递的指令实体
#[derive(Debug)]
pub struct TextInstructEntity {
    pub info: Option<InstructInfoEntity>,
    pub instruct: String,
}

impl TextInstructEntity {
    /// 通过外部请求实体创建内部指令实体
    pub fn create_by_req(req: TextInstruct) -> Self {
        if let Some(info) = req.info {
            TextInstructEntity {
                info: Some(InstructInfoEntity {
                    instruct_type: info.instruct_type(),
                    receive_manipulate_submodule: info.receive_manipulate_submodule,
                }),
                instruct: req.instruct,
            }
        } else {
            TextInstructEntity {
                info: None,
                instruct: req.instruct,
            }
        }
    }

    /// 由指令创建请求实体用于发送
    pub fn create_req(self) -> TextInstruct {
        if let Some(info) = self.info {
            TextInstruct {
                info: Some(InstructInfo {
                    instruct_type: info.instruct_type.into(),
                    receive_manipulate_submodule: info.receive_manipulate_submodule,
                }),
                instruct: self.instruct,
            }
        } else {
            TextInstruct {
                info: None,
                instruct: self.instruct,
            }
        }
    }
}
