use nihility_common::instruct::{InstructReq, InstructType};

/// 核心心模块内部传递的指令实体
#[derive(Debug)]
pub struct InstructEntity {
    pub instruct_type: InstructType,
    pub message: Vec<String>,
}

impl InstructEntity {
    /// 通过外部请求实体创建内部指令实体
    pub fn create_by_req(req: InstructReq) -> Self {
        let instruct_type = match InstructType::from_i32(req.instruct_type) {
            Some(result) => result,
            None => InstructType::DefaultType,
        };
        InstructEntity {
            instruct_type,
            message: req.message,
        }
    }

    /// 由指令创建请求实体用于发送
    pub fn create_req(self) -> InstructReq {
        InstructReq {
            instruct_type: self.instruct_type.into(),
            message: self.message,
        }
    }
}
