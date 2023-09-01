use std::error::Error;

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

#[derive(Debug)]
pub struct Module {
    pub name: String,
    id: u8,
    instruct_client: InstructClient<Channel>,
    manipulate_client: ManipulateClient<Channel>,
}

impl Module {
    pub async fn create_by_req(req: ModuleInfoReq) -> Result<Self, Box<dyn Error>> {
        let mut grpc_addr = "http://".to_string();
        grpc_addr.push_str(req.grpc_addr.as_str());
        let instruct_client = InstructClient::connect(grpc_addr.to_string()).await?;
        let manipulate_client = ManipulateClient::connect(grpc_addr.to_string()).await?;
        Ok(Module {
            name: req.name,
            id: 0,
            instruct_client,
            manipulate_client,
        })
    }

    pub async fn send_instruct(&mut self, instruct: InstructEntity) -> Result<bool, Box<dyn Error>> {
        let instruct_req = InstructReq {
            instruct_type: instruct.instruct_type.into(),
            message: instruct.message,
        };
        let req = Request::new(instruct_req);
        let result = self.instruct_client.send_instruct(req).await?.into_inner();
        Ok(result.status)
    }

    pub async fn send_manipulate(&mut self, manipulate: ManipulateEntity) -> Result<bool, Box<dyn Error>> {
        let manipulate_req = ManipulateReq {
            manipulate_type: manipulate.manipulate_type.into(),
            command: manipulate.command,
        };
        let req = Request::new(manipulate_req);
        let result = self.manipulate_client.send_manipulate(req).await?.into_inner();
        Ok(result.status)
    }
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
