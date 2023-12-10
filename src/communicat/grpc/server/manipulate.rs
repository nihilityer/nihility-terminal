use nihility_common::manipulate::manipulate_server::Manipulate;
use nihility_common::manipulate::{SimpleManipulate, TextDisplayManipulate};
use nihility_common::response_code::{Resp, RespCode};
use tokio::spawn;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status, Streaming};
use tracing::error;

use crate::communicat::grpc::server::StreamResp;
use crate::entity::manipulate::ManipulateEntity;

pub struct ManipulateImpl {
    manipulate_sender: UnboundedSender<ManipulateEntity>,
}

#[tonic::async_trait]
impl Manipulate for ManipulateImpl {
    async fn send_simple_manipulate(
        &self,
        request: Request<SimpleManipulate>,
    ) -> Result<Response<Resp>, Status> {
        match self
            .manipulate_sender
            .send(ManipulateEntity::create_by_simple_type_req(
                request.into_inner(),
            )) {
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
    ) -> Result<Response<Resp>, Status> {
        match self
            .manipulate_sender
            .send(ManipulateEntity::create_by_text_type_req(
                request.into_inner(),
            )) {
            Ok(_) => Ok(Response::new(Resp {
                code: RespCode::Success.into(),
            })),
            Err(e) => {
                error!(
                    "Grpc Manipulate Server send_text_display_manipulate Error: {:?}",
                    &e
                );
                Err(Status::from_error(Box::new(e)))
            }
        }
    }

    type SendMultipleTextDisplayManipulateStream = StreamResp;

    async fn send_multiple_text_display_manipulate(
        &self,
        request: Request<Streaming<TextDisplayManipulate>>,
    ) -> Result<Response<Self::SendMultipleTextDisplayManipulateStream>, Status> {
        let mut req_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(128);
        let manipualte_sender = self.manipulate_sender.clone();
        spawn(async move {
            while let Some(result) = req_stream.next().await {
                match result {
                    Ok(manipulate) => {
                        match manipualte_sender
                            .send(ManipulateEntity::create_by_text_type_req(manipulate))
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
                                        error!("Manipulate Server send_multiple_text_display_manipulate Send To Stream Error: {:?}", e);
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Manipulate Server send_multiple_text_display_manipulate Send To Core Error: {:?}", e);
                                match tx
                                    .send(Ok(Resp {
                                        code: RespCode::UnknownError.into(),
                                    }))
                                    .await
                                {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!("Manipulate Server send_multiple_text_display_manipulate Send To Stream Error: {:?}", e);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            "Manipulate Server send_multiple_text_display_manipulate Receive Error: {:?}",
                            &e
                        );
                        match tx.send(Err(e)).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!("Manipulate Server send_multiple_text_display_manipulate Send To Stream Error: {:?}", e);
                                break;
                            }
                        }
                    }
                }
            }
        });
        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::SendMultipleTextDisplayManipulateStream
        ))
    }
}

impl ManipulateImpl {
    pub fn init(sender: UnboundedSender<ManipulateEntity>) -> Self {
        ManipulateImpl {
            manipulate_sender: sender,
        }
    }
}
