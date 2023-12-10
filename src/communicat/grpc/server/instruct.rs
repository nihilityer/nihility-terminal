use nihility_common::instruct::instruct_server::Instruct;
use nihility_common::instruct::TextInstruct;
use nihility_common::response_code::{Resp, RespCode};
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::ReceiverStream;
use tonic::codegen::tokio_stream::StreamExt;
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
            .send(InstructEntity::create_by_text_type_req(
                request.into_inner(),
            )) {
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
        let mut req_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(128);
        let instruct_sender = self.instruct_sender.clone();
        spawn(async move {
            while let Some(result) = req_stream.next().await {
                match result {
                    Ok(instruct) => {
                        match instruct_sender
                            .send(InstructEntity::create_by_text_type_req(instruct))
                        {
                            Ok(_) => {
                                match tx
                                    .send(Ok(Resp {
                                        code: RespCode::Success.into(),
                                    }))
                                    .await
                                {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!("Instruct Server send_multiple_text_instruct Send To Stream Error: {:?}", e);
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Instruct Server send_multiple_text_instruct Send To Core Error: {:?}", e);
                                match tx
                                    .send(Ok(Resp {
                                        code: RespCode::UnknownError.into(),
                                    }))
                                    .await
                                {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!("Instruct Server send_multiple_text_instruct Send To Stream Error: {:?}", e);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            "Instruct Server send_multiple_text_instruct Receive Error: {:?}",
                            &e
                        );
                        match tx.send(Err(e)).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("Instruct Server send_multiple_text_instruct Send To Stream Error: {:?}", e);
                                break;
                            }
                        }
                    }
                }
            }
        });
        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::SendMultipleTextInstructStream
        ))
    }
}
