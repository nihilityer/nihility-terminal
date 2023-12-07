use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use nihility_common::instruct::instruct_client::InstructClient;
use nihility_common::manipulate::manipulate_client::ManipulateClient;
use nihility_common::submodule::ReceiveType;
use tokio::spawn;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error};

use crate::communicat::mock::{MockInstructClient, MockManipulateClient};
use crate::communicat::{SendInstructOperate, SendManipulateOperate};
use crate::config::GrpcConfig;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::SimpleManipulateEntity;
use crate::entity::submodule::{ModuleOperate, Submodule};
use crate::CANCELLATION_TOKEN;

mod client;
mod server;

const GRPC_CONN_ADDR_FIELD: &str = "grpc_addr";

pub(super) fn start(
    grpc_config: GrpcConfig,
    communicat_status_sender: UnboundedSender<String>,
    operate_module_sender: UnboundedSender<ModuleOperate>,
    instruct_sender: UnboundedSender<InstructEntity>,
    manipulate_sender: UnboundedSender<SimpleManipulateEntity>,
) {
    spawn(async move {
        if let Err(e) = server::start_server(
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

/// 创建grpc通信的子模块
///
/// 连接参数需要具有`grpc_addr`参数
pub(crate) async fn create_grpc_module(operate: ModuleOperate) -> Result<Submodule> {
    debug!("Start Create Grpc Submodule By {:?}", &operate);
    if operate.conn_params.get(GRPC_CONN_ADDR_FIELD).is_none() {
        return Err(anyhow!(
            "Create {:?} Type Submodule Error, ModuleOperate Missing {:?} Filed",
            &operate.submodule_type,
            GRPC_CONN_ADDR_FIELD
        ));
    }
    let grpc_addr = operate.conn_params.get(GRPC_CONN_ADDR_FIELD).unwrap();

    let mut instruct_client: Box<dyn SendInstructOperate + Send + Sync> =
        Box::<MockInstructClient>::default();
    let mut manipulate_client: Box<dyn SendManipulateOperate + Send + Sync> =
        Box::<MockManipulateClient>::default();
    match operate.receive_type {
        ReceiveType::DefaultType => {
            instruct_client = Box::new(InstructClient::connect(grpc_addr.to_string()).await?);
            manipulate_client = Box::new(ManipulateClient::connect(grpc_addr.to_string()).await?);
        }
        ReceiveType::JustInstructType => {
            instruct_client = Box::new(InstructClient::connect(grpc_addr.to_string()).await?);
        }
        ReceiveType::JustManipulateType => {
            manipulate_client = Box::new(ManipulateClient::connect(grpc_addr.to_string()).await?);
        }
    }

    let mut instruct_map = HashMap::<String, String>::new();
    for instruct in operate.default_instruct {
        instruct_map.insert(instruct, String::new());
    }
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    debug!("Create Grpc Submodule {:?} Success", &operate.name);
    Ok(Submodule {
        name: operate.name,
        default_instruct_map: instruct_map,
        sub_module_type: operate.submodule_type,
        receive_type: operate.receive_type,
        heartbeat_time: timestamp,
        instruct_client,
        manipulate_client,
    })
}
