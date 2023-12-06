use nihility_common::manipulate::manipulate_server::Manipulate;
use nihility_common::manipulate::{SimpleManipulate, TextDisplayManipulate};
use nihility_common::response_code::{Resp, RespCode};
use tokio::sync::mpsc::UnboundedSender;
use tonic::{Request, Response, Status, Streaming};
use tracing::error;

use crate::communicat::grpc::server::StreamResp;
use crate::entity::manipulate::SimpleManipulateEntity;

pub struct ManipulateImpl {
    manipulate_sender: UnboundedSender<SimpleManipulateEntity>,
}

#[tonic::async_trait]
impl Manipulate for ManipulateImpl {
    async fn send_simple_manipulate(
        &self,
        request: Request<SimpleManipulate>,
    ) -> std::result::Result<Response<Resp>, Status> {
        match self
            .manipulate_sender
            .send(SimpleManipulateEntity::create_by_req(request.into_inner()))
        {
            Ok(_) => Ok(Response::new(Resp {
                code: RespCode::Success.into(),
            })),
            Err(e) => {
                error!(
                    "Grpc Manipulate Server send_simple_manipulate Error: {:?}",
                    &e
                );
                Err(Status::from_error(Box::new(e)))
            }
        }
    }

    async fn send_text_display_manipulate(
        &self,
        request: Request<TextDisplayManipulate>,
    ) -> std::result::Result<Response<Resp>, Status> {
        todo!()
    }

    type SendMultipleTextDisplayManipulateStream = StreamResp;

    async fn send_multiple_text_display_manipulate(
        &self,
        request: Request<Streaming<TextDisplayManipulate>>,
    ) -> std::result::Result<Response<Self::SendMultipleTextDisplayManipulateStream>, Status> {
        todo!()
    }
}

impl ManipulateImpl {
    pub fn init(sender: UnboundedSender<SimpleManipulateEntity>) -> Self {
        ManipulateImpl {
            manipulate_sender: sender,
        }
    }
}