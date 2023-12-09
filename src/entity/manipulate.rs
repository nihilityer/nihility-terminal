use anyhow::{anyhow, Result};
use nihility_common::manipulate::{
    ManipulateInfo, ManipulateType, SimpleManipulate, TextDisplayManipulate,
};

#[derive(Debug, Clone)]
pub struct ManipulateInfoEntity {
    pub manipulate_type: ManipulateType,
    pub use_module_name: String,
}

#[derive(Debug)]
pub enum ManipulateData {
    Text(String),
    None,
}

/// 核心模块内部传递的操作实体
#[derive(Debug)]
pub struct ManipulateEntity {
    pub info: Option<ManipulateInfoEntity>,
    pub manipulate: ManipulateData,
}

impl ManipulateInfoEntity {
    pub fn create_by_req_info(req_info: Option<ManipulateInfo>) -> Option<Self> {
        match req_info {
            None => None,
            Some(info) => Some(ManipulateInfoEntity {
                manipulate_type: info.manipulate_type(),
                use_module_name: info.use_module_name,
            }),
        }
    }

    pub fn create_req_info(self) -> ManipulateInfo {
        ManipulateInfo {
            manipulate_type: self.manipulate_type.into(),
            use_module_name: self.use_module_name,
        }
    }
}

impl ManipulateEntity {
    pub fn create_by_simple_type_req(req: SimpleManipulate) -> Self {
        ManipulateEntity {
            info: ManipulateInfoEntity::create_by_req_info(req.info),
            manipulate: ManipulateData::None,
        }
    }

    pub fn create_by_text_type_req(req: TextDisplayManipulate) -> Self {
        ManipulateEntity {
            info: ManipulateInfoEntity::create_by_req_info(req.info),
            manipulate: ManipulateData::Text(req.text),
        }
    }

    pub fn create_simple_type_req(self) -> Result<SimpleManipulate> {
        match self.manipulate {
            ManipulateData::None => match self.info {
                None => Ok(SimpleManipulate { info: None }),
                Some(info) => Ok(SimpleManipulate {
                    info: Some(info.create_req_info()),
                }),
            },
            data_type => Err(anyhow!(
                "This Manipulate Entity Have Manipulate Data, Please Create {:?} Type Req",
                data_type
            )),
        }
    }

    pub fn create_text_type_req(self) -> Result<TextDisplayManipulate> {
        match self.manipulate {
            ManipulateData::Text(manipulate) => match self.info {
                None => Ok(TextDisplayManipulate {
                    info: None,
                    text: manipulate,
                }),
                Some(info) => Ok(TextDisplayManipulate {
                    info: Some(info.create_req_info()),
                    text: manipulate,
                }),
            },
            data_type => Err(anyhow!(
                "This Manipulate Entity Is In Other Type, Please Create {:?} Type Req",
                data_type
            )),
        }
    }
}
