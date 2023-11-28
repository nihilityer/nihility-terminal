use nihility_common::instruct::{InstructReq, InstructType};

/// 核心心模块内部传递的指令实体
#[derive(Debug)]
pub struct InstructEntity {
    pub instruct_type: InstructType,
    pub instruct: String,
    pub receive_manipulate_submodule: String,
}

impl InstructEntity {
    /// 通过外部请求实体创建内部指令实体
    pub fn create_by_req(req: InstructReq) -> Self {
        InstructEntity {
            instruct_type: req.instruct_type(),
            instruct: req.instruct,
            receive_manipulate_submodule: req.receive_manipulate_submodule,
        }
    }

    /// 由指令创建请求实体用于发送
    pub fn create_req(self) -> InstructReq {
        InstructReq {
            instruct_type: self.instruct_type.into(),
            instruct: self.instruct,
            receive_manipulate_submodule: self.receive_manipulate_submodule,
        }
    }
}
