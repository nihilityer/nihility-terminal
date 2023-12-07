use nihility_common::instruct::instruct_server::Instruct;
use nihility_common::instruct::TextInstruct;
use nihility_common::response_code::{Resp, RespCode};
use tokio::sync::mpsc::UnboundedSender;
use tonic::{Request, Response, Status, Streaming};
use tracing::error;

use crate::communicat::grpc::server::StreamResp;
use crate::entity::instruct::InstructEntity;

pub struct InstructImpl {
    instruct_sender: UnboundedSender<InstructEntity>,
}

impl InstructImpl {
    pub fn init(sender: UnboundedSender<InstructEntity>) -> Self {
        InstructImpl {
            instruct_sender: sender,
        }
    }
}

#[tonic::async_trait]
impl Instruct for InstructImpl {
    async fn send_text_instruct(
        &self,
        request: Request<TextInstruct>,
    ) -> Result<Response<Resp>, Status> {
        match self
            .instruct_sender
            .send(InstructEntity::create_by_req(request.into_inner()))
        {
            Ok(_) => Ok(Response::new(Resp {
                code: RespCode::Success.into(),
            })),
            Err(e) => {
                error!("Grpc Instruct Server send_text_instruct Error: {:?}", &e);
                Err(Status::from_error(Box::new(e)))
            }
        }
    }

    type SendMultipleTextInstructStream = StreamResp;

    async fn send_multiple_text_instruct(
        &self,
        request: Request<Streaming<TextInstruct>>,
    ) -> Result<Response<Self::SendMultipleTextInstructStream>, Status> {
        todo!()
    }
}
