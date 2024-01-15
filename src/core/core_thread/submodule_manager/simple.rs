use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use nihility_common::{ModuleOperate, OperateType};
use tokio::spawn;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::core::instruct_encoder::InstructEncoder;
use crate::core::instruct_matcher::{InstructMatcher, PointPayload};
use crate::core::submodule_store::SubmoduleStore;
use crate::entity::submodule::Submodule;
use crate::{CANCELLATION_TOKEN, CLOSE_SENDER};

pub fn simple_submodule_manager_thread(
    instruct_encoder: Arc<Box<dyn InstructEncoder + Send + Sync>>,
    instruct_matcher: Arc<Box<dyn InstructMatcher + Send + Sync>>,
    submodule_store: Arc<Mutex<Box<dyn SubmoduleStore + Send + Sync>>>,
    module_operate_receiver: UnboundedReceiver<ModuleOperate>,
) -> Result<()> {
    let close_sender = CLOSE_SENDER.get().unwrap().upgrade().unwrap();
    spawn(async move {
        if let Err(e) = start(
            instruct_encoder,
            instruct_matcher,
            submodule_store,
            module_operate_receiver,
        )
        .await
        {
            error!("Simple Submodule Manager Thread Error: {}", e);
            CANCELLATION_TOKEN.cancel();
        }
        close_sender
            .send("Simple Submodule Manager Thread".to_string())
            .await
            .unwrap();
    });
    Ok(())
}

async fn start(
    instruct_encoder: Arc<Box<dyn InstructEncoder + Send + Sync>>,
    instruct_matcher: Arc<Box<dyn InstructMatcher + Send + Sync>>,
    submodule_store: Arc<Mutex<Box<dyn SubmoduleStore + Send + Sync>>>,
    mut module_operate_receiver: UnboundedReceiver<ModuleOperate>,
) -> Result<()> {
    info!("Simple Submodule Manager Thread Start");
    while let Some(module_operate) = module_operate_receiver.recv().await {
        match module_operate.operate_type {
            OperateType::Register => match register_submodule(
                instruct_encoder.clone(),
                instruct_matcher.clone(),
                submodule_store.clone(),
                module_operate,
            )
            .await
            {
                Ok(register_submodule_name) => {
                    info!("Register Submodule {:?} success", register_submodule_name);
                }
                Err(e) => {
                    error!("Register Submodule Error: {}", e)
                }
            },
            OperateType::Offline => match offline_submodule(
                instruct_matcher.clone(),
                submodule_store.clone(),
                module_operate,
            )
            .await
            {
                Ok(offline_submodule_name) => {
                    info!("Offline Submodule {:?} success", offline_submodule_name);
                }
                Err(e) => {
                    error!("Offline Submodule Error: {}", e)
                }
            },
            OperateType::Heartbeat => {
                // 此方法内部抛出的错误无法忽略
                update_submodule_heartbeat(submodule_store.clone(), module_operate).await?;
            }
            OperateType::Update => match update_submodule(
                instruct_encoder.clone(),
                instruct_matcher.clone(),
                submodule_store.clone(),
                module_operate,
            )
            .await
            {
                Ok(update_submodule_name) => {
                    info!("Update Submodule {:?} success", update_submodule_name);
                }
                Err(e) => {
                    error!("Update Submodule Error: {}", e)
                }
            },
            OperateType::Undefined => {
                error!("OperateType Undefined")
            }
        }
    }
    Ok(())
}

async fn update_submodule(
    instruct_encoder: Arc<Box<dyn InstructEncoder + Send + Sync>>,
    instruct_matcher: Arc<Box<dyn InstructMatcher + Send + Sync>>,
    submodule_store: Arc<Mutex<Box<dyn SubmoduleStore + Send + Sync>>>,
    module_operate: ModuleOperate,
) -> Result<String> {
    info!(
        "Update Submodule {:?} Default Instruct",
        &module_operate.name
    );
    let mut new_instruct = HashMap::<String, Vec<f32>>::new();
    let mut update_instruct_map = HashMap::<String, String>::new();
    let mut remove_point_ids = Vec::<String>::new();
    // 确认子模块指令新增的指令，获取去除指令的point_id，没有变化的指令直接获取point_id
    if let Some(submodule) = submodule_store
        .lock()
        .await
        .get_mut(&module_operate.name)
        .await?
    {
        let mut retain_instruct = Vec::<String>::new();
        for instruct in module_operate.info.unwrap().default_instruct.iter() {
            match submodule.default_instruct_map.get(instruct.as_str()) {
                None => {
                    new_instruct.insert(instruct.to_string(), Vec::new());
                }
                Some(_) => {
                    retain_instruct.push(instruct.to_string());
                }
            }
            if submodule.default_instruct_map.len() > retain_instruct.len() {
                for instruct in &retain_instruct {
                    if let Some(point_id) = submodule.default_instruct_map.get(instruct.as_str()) {
                        update_instruct_map.insert(instruct.to_string(), point_id.to_string());
                        submodule.default_instruct_map.remove(instruct.as_str());
                    }
                }
            }
            for (_, point_id) in submodule.default_instruct_map.iter() {
                remove_point_ids.push(point_id.to_string())
            }
        }
    }
    // 将新增指令编码
    for (instruct, _) in new_instruct.clone() {
        new_instruct.insert(instruct.to_string(), instruct_encoder.encode(&instruct)?);
    }
    // 将新增的指令分配的point_id存入，然后在qdrant上移除需要删除指令对应的点，最后插入新增指令的点
    let mut insert_points = Vec::<PointPayload>::new();
    for (instruct, encode_result) in new_instruct {
        let id = Uuid::new_v4();
        update_instruct_map.insert(instruct.to_string(), id.to_string());
        insert_points.push(PointPayload {
            uuid: id.to_string(),
            instruct: instruct.to_string(),
            encode: encode_result.clone(),
        });
    }
    instruct_matcher
        .append_points(module_operate.name.to_string(), insert_points)
        .await?;
    instruct_matcher.remove_points(remove_point_ids).await?;
    if let Some(submodule) = submodule_store
        .lock()
        .await
        .get_mut(&module_operate.name)
        .await?
    {
        submodule.default_instruct_map = update_instruct_map;
    }
    Ok(module_operate.name.to_string())
}

async fn update_submodule_heartbeat(
    submodule_store: Arc<Mutex<Box<dyn SubmoduleStore + Send + Sync>>>,
    module_operate: ModuleOperate,
) -> Result<()> {
    debug!("Submodule {:?} Heartbeat", &module_operate.name);
    if let Some(submodule) = submodule_store
        .lock()
        .await
        .get_mut(&module_operate.name)
        .await?
    {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        submodule.heartbeat_time = timestamp;
    }
    Ok(())
}

async fn offline_submodule(
    instruct_matcher: Arc<Box<dyn InstructMatcher + Send + Sync>>,
    submodule_store: Arc<Mutex<Box<dyn SubmoduleStore + Send + Sync>>>,
    module_operate: ModuleOperate,
) -> Result<String> {
    info!("Offline Submodule {:?}", &module_operate.name);
    let mut point_ids = Vec::<String>::new();
    {
        if let Some(submodule) = submodule_store
            .lock()
            .await
            .get(&module_operate.name)
            .await?
        {
            for (_, point_id) in submodule.default_instruct_map.iter() {
                point_ids.push(point_id.to_string());
            }
        }
    }
    instruct_matcher.remove_points(point_ids).await?;
    Ok(module_operate.name.to_string())
}

async fn register_submodule(
    instruct_encoder: Arc<Box<dyn InstructEncoder + Send + Sync>>,
    instruct_matcher: Arc<Box<dyn InstructMatcher + Send + Sync>>,
    submodule_store: Arc<Mutex<Box<dyn SubmoduleStore + Send + Sync>>>,
    module_operate: ModuleOperate,
) -> Result<String> {
    info!("start register model：{:?}", &module_operate.name);
    let register_submodule_name = module_operate.name.to_string();
    let mut submodule = Submodule::create(&module_operate).await?;
    let mut points = Vec::<PointPayload>::new();
    if let Some(_) = submodule_store
        .lock()
        .await
        .get(&module_operate.name)
        .await?
    {
        return Err(anyhow!(
            "The Current Submodule {:?} Is Registered",
            &module_operate.name
        ));
    }
    let original_default_instruct_map = submodule.default_instruct_map.clone();
    for (instruct, _) in original_default_instruct_map.iter() {
        let encode_result = instruct_encoder.encode(instruct)?;
        let id = Uuid::new_v4();
        submodule
            .default_instruct_map
            .insert(instruct.to_string(), id.to_string());
        points.push(PointPayload {
            encode: encode_result.clone(),
            instruct: instruct.to_string(),
            uuid: id.to_string(),
        });
    }

    submodule_store.lock().await.insert(submodule).await?;
    instruct_matcher
        .append_points(register_submodule_name.to_string(), points)
        .await?;
    Ok(register_submodule_name)
}
