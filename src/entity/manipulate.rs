use nihility_common::manipulate::{ManipulateInfo, ManipulateType, SimpleManipulate};

#[derive(Debug)]
pub struct ManipulateInfoEntity {
    pub manipulate_type: ManipulateType,
    pub use_module_name: String,
}

/// 核心模块内部传递的操作实体
#[derive(Debug)]
pub struct SimpleManipulateEntity {
    pub info: Option<ManipulateInfoEntity>,
}

impl ManipulateInfoEntity {
    pub fn create_simple_manipulate(self) -> SimpleManipulate {
        SimpleManipulate {
            info: Some(ManipulateInfo {
                manipulate_type: self.manipulate_type.into(),
                use_module_name: self.use_module_name,
            })
        }
    }

    pub fn create_simple_manipulate_entity(self) -> SimpleManipulateEntity {
        SimpleManipulateEntity {
            info: Some(self)
        }
    }
}

impl SimpleManipulateEntity {
    /// 通过外部请求实体创建内部操作实体
    pub fn create_by_req(req: SimpleManipulate) -> Self {
        if let Some(info) = req.info {
            SimpleManipulateEntity {
                info: Some(ManipulateInfoEntity {
                    manipulate_type: info.manipulate_type(),
                    use_module_name: info.use_module_name,
                }),
            }
        } else {
            SimpleManipulateEntity { info: None }
        }
    }

    /// 有操作创建请求实体用于发送
    pub fn create_req(self) -> SimpleManipulate {
        if let Some(info) = self.info {
            SimpleManipulate {
                info: Some(ManipulateInfo {
                    manipulate_type: info.manipulate_type.into(),
                    use_module_name: info.use_module_name,
                }),
            }
        } else {
            SimpleManipulate { info: None }
        }
    }
}
