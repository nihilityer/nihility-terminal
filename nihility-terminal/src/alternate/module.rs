use tonic::{Request, transport::Channel};

use nihility_common::{
    instruct::{
        instruct_client::InstructClient,
        InstructReq,
        InstructType,
    },
    manipulate::{
        manipulate_client::ManipulateClient,
        ManipulateReq,
        ManipulateType,
    },
    module_info::ModuleInfoReq,
};

use crate::AppError;

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    instruct_client: InstructClient<Channel>,
    manipulate_client: ManipulateClient<Channel>,
}

#[derive(Debug)]
pub struct InstructEntity {
    pub instruct_type: InstructType,
    pub message: Vec<String>,
}

#[derive(Debug)]
pub struct ManipulateEntity {
    pub manipulate_type: ManipulateType,
    pub command: String,
}

impl Module {
    pub async fn create_by_req(req: ModuleInfoReq) -> Result<Self, AppError> {
        let mut grpc_addr = "http://".to_string();
        grpc_addr.push_str(req.grpc_addr.as_str());
        let instruct_client = InstructClient::connect(grpc_addr.to_string()).await?;
        let manipulate_client = ManipulateClient::connect(grpc_addr.to_string()).await?;
        Ok(Module {
            name: req.name,
            instruct_client,
            manipulate_client,
        })
    }

    pub async fn send_instruct(&mut self, instruct: InstructEntity) -> Result<bool, AppError> {
        let instruct_req = InstructReq {
            instruct_type: instruct.instruct_type.into(),
            message: instruct.message,
        };
        let req = Request::new(instruct_req);
        let result = self.instruct_client.send_instruct(req).await?.into_inner();
        Ok(result.status)
    }

    pub async fn send_manipulate(&mut self, manipulate: ManipulateEntity) -> Result<bool, AppError> {
        let manipulate_req = ManipulateReq {
            manipulate_type: manipulate.manipulate_type.into(),
            command: manipulate.command,
        };
        let req = Request::new(manipulate_req);
        let result = self.manipulate_client.send_manipulate(req).await?.into_inner();
        Ok(result.status)
    }
}

impl InstructEntity {
    pub fn create_by_req(req: InstructReq) -> Self {
        let instruct_type = match InstructType::from_i32(req.instruct_type).ok_or(InstructType::DefaultType) {
            Ok(result) => {
                result
            },
            Err(_) => {
                InstructType::DefaultType
            }
        };
        InstructEntity {
            instruct_type,
            message: req.message,
        }
    }
}

impl ManipulateEntity {
    pub fn create_by_req(req: ManipulateReq) -> Self {
        let manipulate_type = match ManipulateType::from_i32(req.manipulate_type).ok_or(ManipulateType::DefaultType) {
            Ok(result) => {
                result
            },
            Err(_) => {
                ManipulateType::DefaultType
            }
        };
        ManipulateEntity {
            manipulate_type,
            command: req.command,
        }
    }
}
