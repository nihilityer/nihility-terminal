use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use nihility_common::instruct::instruct_client::InstructClient;
use nihility_common::instruct::instruct_server::{Instruct, InstructServer};
use nihility_common::instruct::TextInstruct;
use nihility_common::manipulate::manipulate_client::ManipulateClient;
use nihility_common::manipulate::manipulate_server::{Manipulate, ManipulateServer};
use nihility_common::manipulate::{SimpleManipulate, TextDisplayManipulate};
use nihility_common::response_code::{Resp, RespCode};
use nihility_common::submodule::submodule_server::{Submodule, SubmoduleServer};
use nihility_common::submodule::SubmoduleReq;
use tokio::spawn;
use tokio::sync::mpsc::UnboundedSender;
use tonic::codegen::tokio_stream::Stream;
use tonic::transport::{Channel, Server};
use tonic::{Request, Response, Status, Streaming};
use tracing::{error, info};

use crate::communicat::{SendInstructOperate, SendManipulateOperate};
use crate::config::GrpcConfig;
use crate::entity::instruct::TextInstructEntity;
use crate::entity::manipulate::SimpleManipulateEntity;
use crate::entity::submodule::{ModuleOperate, OperateType};
use crate::CANCELLATION_TOKEN;

type StreamResp = Pin<Box<dyn Stream<Item = Result<Resp, Status>> + Send>>;

pub(super) fn start(
    grpc_config: GrpcConfig,
    communicat_status_sender: UnboundedSender<String>,
    operate_module_sender: UnboundedSender<ModuleOperate>,
    instruct_sender: UnboundedSender<TextInstructEntity>,
    manipulate_sender: UnboundedSender<SimpleManipulateEntity>,
) {
    spawn(async move {
        if let Err(e) = start_server(
            grpc_config,
            operate_module_sender,
            instruct_sender,
            manipulate_sender,
        )
        .await
        {
            error!("Grpc Server Error: {}", e);
            CANCELLATION_TOKEN.cancel();
        }
        communicat_status_sender
            .send("Grpc Server".to_string())
            .unwrap();
    });
}

async fn start_server(
    grpc_config: GrpcConfig,
    operate_module_sender: UnboundedSender<ModuleOperate>,
    instruct_sender: UnboundedSender<TextInstructEntity>,
    manipulate_sender: UnboundedSender<SimpleManipulateEntity>,
) -> Result<()> {
    if !grpc_config.enable {
        return Ok(());
    }
    let bind_addr = format!("{}:{}", grpc_config.addr, grpc_config.port);
    info!("Grpc Server Bind At {}", &bind_addr);

    Server::builder()
        .add_service(SubmoduleServer::new(SubmoduleImpl::init(
            operate_module_sender,
        )))
        .add_service(InstructServer::new(InstructImpl::init(instruct_sender)))
        .add_service(ManipulateServer::new(ManipulateImpl::init(
            manipulate_sender,
        )))
        .serve_with_shutdown(bind_addr.parse()?, async move {
            CANCELLATION_TOKEN.cancelled().await
        })
        .await?;

    Ok(())
}

#[async_trait]
impl SendInstructOperate for InstructClient<Channel> {
    async fn send(&mut self, instruct: TextInstruct) -> Result<RespCode> {
        Ok(self
            .send_text_instruct(Request::new(instruct))
            .await?
            .into_inner()
            .code())
    }
}

#[async_trait]
impl SendManipulateOperate for ManipulateClient<Channel> {
    async fn send(&mut self, manipulate: SimpleManipulate) -> Result<RespCode> {
        Ok(self
            .send_simple_manipulate(Request::new(manipulate))
            .await?
            .into_inner()
            .code())
    }
}

pub struct SubmoduleImpl {
    operate_module_sender: UnboundedSender<ModuleOperate>,
}

pub struct InstructImpl {
    instruct_sender: UnboundedSender<TextInstructEntity>,
}

pub struct ManipulateImpl {
    manipulate_sender: UnboundedSender<SimpleManipulateEntity>,
}

#[tonic::async_trait]
impl Submodule for SubmoduleImpl {
    async fn register(&self, request: Request<SubmoduleReq>) -> Result<Response<Resp>, Status> {
        self.operate_module_sender
            .send(ModuleOperate::create_by_req(
                request.into_inner(),
                OperateType::REGISTER,
            ))
            .unwrap();
        Ok(Response::new(Resp {
            code: RespCode::Success.into(),
        }))
    }

    async fn offline(&self, request: Request<SubmoduleReq>) -> Result<Response<Resp>, Status> {
        self.operate_module_sender
            .send(ModuleOperate::create_by_req(
                request.into_inner(),
                OperateType::OFFLINE,
            ))
            .unwrap();
        Ok(Response::new(Resp {
            code: RespCode::Success.into(),
        }))
    }

    async fn heartbeat(&self, request: Request<SubmoduleReq>) -> Result<Response<Resp>, Status> {
        self.operate_module_sender
            .send(ModuleOperate::create_by_req(
                request.into_inner(),
                OperateType::HEARTBEAT,
            ))
            .unwrap();
        Ok(Response::new(Resp {
            code: RespCode::Success.into(),
        }))
    }

    async fn update(&self, request: Request<SubmoduleReq>) -> Result<Response<Resp>, Status> {
        self.operate_module_sender
            .send(ModuleOperate::create_by_req(
                request.into_inner(),
                OperateType::UPDATE,
            ))
            .unwrap();
        Ok(Response::new(Resp {
            code: RespCode::Success.into(),
        }))
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
            .send(TextInstructEntity::create_by_req(request.into_inner()))
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
    ) -> std::result::Result<Response<Self::SendMultipleTextInstructStream>, Status> {
        todo!()
    }
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

impl SubmoduleImpl {
    pub fn init(operate_module_sender: UnboundedSender<ModuleOperate>) -> Self {
        SubmoduleImpl {
            operate_module_sender,
        }
    }
}

impl InstructImpl {
    pub fn init(sender: UnboundedSender<TextInstructEntity>) -> Self {
        InstructImpl {
            instruct_sender: sender,
        }
    }
}

impl ManipulateImpl {
    pub fn init(sender: UnboundedSender<SimpleManipulateEntity>) -> Self {
        ManipulateImpl {
            manipulate_sender: sender,
        }
    }
}
