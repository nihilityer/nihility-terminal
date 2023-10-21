use nihility_common::manipulate::{ManipulateReq, ManipulateType};

/// 核心模块内部传递的操作实体
#[derive(Debug)]
pub struct ManipulateEntity {
    pub manipulate_type: ManipulateType,
    pub command: String,
}

impl ManipulateEntity {
    /// 通过外部请求实体创建内部操作实体
    pub fn create_by_req(req: ManipulateReq) -> Self {
        let manipulate_type = match ManipulateType::from_i32(req.manipulate_type) {
            Some(result) => result,
            None => ManipulateType::DefaultType,
        };
        ManipulateEntity {
            manipulate_type,
            command: req.command,
        }
    }

    /// 有操作创建请求实体用于发送
    pub fn create_req(self) -> ManipulateReq {
        ManipulateReq {
            manipulate_type: self.manipulate_type.into(),
            command: self.command,
        }
    }
}
