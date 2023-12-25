use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use nihility_common::{ModuleOperate, OperateType};
use tokio::spawn;
use tokio::sync::mpsc::UnboundedReceiver;
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
    submodule_store: Arc<Box<dyn SubmoduleStore + Send + Sync>>,
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

/// 负责管理子模块
///
/// 1、定时获取子模块心跳，当离线时将对应子模块从模组中卸载
///
/// 2、特定错误进行重试或只通知
async fn start(
    instruct_encoder: Arc<Box<dyn InstructEncoder + Send + Sync>>,
    instruct_matcher: Arc<Box<dyn InstructMatcher + Send + Sync>>,
    submodule_store: Arc<Box<dyn SubmoduleStore + Send + Sync>>,
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

/// 更新子模块指令设置，如果要更新连接设置应该先离线然后注册，而不是发送更新操作
async fn update_submodule(
    instruct_encoder: Arc<Box<dyn InstructEncoder + Send + Sync>>,
    instruct_matcher: Arc<Box<dyn InstructMatcher + Send + Sync>>,
    submodule_store: Arc<Box<dyn SubmoduleStore + Send + Sync>>,
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
    if let Some(submodule) = submodule_store.get_and_remove(&module_operate.name).await? {
        let mut retain_instruct = Vec::<String>::new();
        for instruct in module_operate.info.unwrap().default_instruct.iter() {
            match submodule.default_instruct_map.lock() {
                Ok(mut default_instruct_map) => {
                    match default_instruct_map.get(instruct.as_str()) {
                        None => {
                            new_instruct.insert(instruct.to_string(), Vec::new());
                        }
                        Some(_) => {
                            retain_instruct.push(instruct.to_string());
                        }
                    }
                    if default_instruct_map.len() > retain_instruct.len() {
                        for instruct in &retain_instruct {
                            if let Some(point_id) = default_instruct_map.get(instruct.as_str()) {
                                update_instruct_map
                                    .insert(instruct.to_string(), point_id.to_string());
                                default_instruct_map.remove(instruct.as_str());
                            }
                        }
                    }
                    for (_, point_id) in default_instruct_map.iter() {
                        remove_point_ids.push(point_id.to_string())
                    }
                }
                Err(e) => {
                    error!(
                        "Lock {:?} default_instruct_map Error: {}",
                        &submodule.name, e
                    );
                }
            }
        }
        submodule_store.insert(submodule).await?;
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
    if let Some(submodule) = submodule_store.get_and_remove(&module_operate.name).await? {
        match submodule.default_instruct_map.lock() {
            Ok(mut default_instruct_map) => {
                *default_instruct_map = update_instruct_map;
            }
            Err(e) => {
                error!(
                    "Lock {:?} default_instruct_map Error: {}",
                    &submodule.name, e
                );
            }
        }
        submodule_store.insert(submodule).await?;
    }
    Ok(module_operate.name.to_string())
}

/// 更新子模块心跳时间
async fn update_submodule_heartbeat(
    submodule_store: Arc<Box<dyn SubmoduleStore + Send + Sync>>,
    module_operate: ModuleOperate,
) -> Result<()> {
    debug!("Submodule {:?} Heartbeat", &module_operate.name);
    if let Some(submodule) = submodule_store.get_and_remove(&module_operate.name).await? {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        match submodule.heartbeat_time.write() {
            Ok(mut heartbeat_time) => {
                *heartbeat_time = timestamp;
            }
            Err(e) => {
                error!("Lock {:?} heartbeat_time Error: {}", &submodule.name, e);
            }
        }
        submodule_store.insert(submodule).await?;
    }
    Ok(())
}

/// 离线子模块处理
async fn offline_submodule(
    instruct_matcher: Arc<Box<dyn InstructMatcher + Send + Sync>>,
    submodule_store: Arc<Box<dyn SubmoduleStore + Send + Sync>>,
    module_operate: ModuleOperate,
) -> Result<String> {
    info!("Offline Submodule {:?}", &module_operate.name);
    let mut point_ids = Vec::<String>::new();
    {
        if let Some(submodule) = submodule_store.get_and_remove(&module_operate.name).await? {
            match submodule.default_instruct_map.lock() {
                Ok(default_instruct_map) => {
                    for (_, point_id) in default_instruct_map.iter() {
                        point_ids.push(point_id.to_string());
                    }
                }
                Err(e) => {
                    error!(
                        "Lock {:?} default_instruct_map Error: {}",
                        &submodule.name, e
                    );
                }
            }
        }
    }
    instruct_matcher.remove_points(point_ids).await?;
    Ok(module_operate.name.to_string())
}

/// 注册子模块处理
async fn register_submodule(
    instruct_encoder: Arc<Box<dyn InstructEncoder + Send + Sync>>,
    instruct_matcher: Arc<Box<dyn InstructMatcher + Send + Sync>>,
    submodule_store: Arc<Box<dyn SubmoduleStore + Send + Sync>>,
    module_operate: ModuleOperate,
) -> Result<String> {
    info!("start register model：{:?}", &module_operate.name);
    let register_submodule_name = module_operate.name.to_string();
    let submodule = Submodule::create(&module_operate).await?;
    let mut points = Vec::<PointPayload>::new();
    if let Some(submodule) = submodule_store.get_and_remove(&module_operate.name).await? {
        submodule_store.insert(submodule).await?;
        return Err(anyhow!(
            "The Current Submodule {:?} Is Registered",
            &module_operate.name
        ));
    }
    match submodule.default_instruct_map.lock() {
        Ok(mut default_instruct_map) => {
            let original_default_instruct_map = default_instruct_map.clone();
            for (instruct, _) in original_default_instruct_map.iter() {
                let encode_result = instruct_encoder.encode(instruct)?;
                let id = Uuid::new_v4();
                default_instruct_map.insert(instruct.to_string(), id.to_string());
                points.push(PointPayload {
                    encode: encode_result.clone(),
                    instruct: instruct.to_string(),
                    uuid: id.to_string(),
                });
            }
        }
        Err(e) => {
            error!(
                "Lock {:?} default_instruct_map Error: {}",
                &submodule.name, e
            );
        }
    }

    submodule_store.insert(submodule).await?;
    instruct_matcher
        .append_points(register_submodule_name.to_string(), points)
        .await?;
    Ok(register_submodule_name)
}