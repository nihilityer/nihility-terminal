use nihility_common::manipulate::{ManipulateReq, ManipulateType};

/// 核心模块内部传递的操作实体
#[derive(Debug)]
pub struct ManipulateEntity {
    pub manipulate_type: ManipulateType,
    pub command: String,
    pub use_module_name: String,
}

impl ManipulateEntity {
    /// 通过外部请求实体创建内部操作实体
    pub fn create_by_req(req: ManipulateReq) -> Self {
        ManipulateEntity {
            manipulate_type: req.manipulate_type(),
            command: req.command,
            use_module_name: req.use_module_name,
        }
    }

    /// 有操作创建请求实体用于发送
    pub fn create_req(self) -> ManipulateReq {
        ManipulateReq {
            manipulate_type: self.manipulate_type.into(),
            command: self.command,
            use_module_name: self.use_module_name,
        }
    }
}
