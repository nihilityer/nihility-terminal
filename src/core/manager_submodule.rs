use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::core::instruct_manager::PointPayload;
use crate::entity::module::{ModuleOperate, OperateType, Submodule};

use super::{INSTRUCT_ENCODER, INSTRUCT_MANAGER, SUBMODULE_MAP};

/// 负责管理子模块
///
/// 1、定时获取子模块心跳，当离线时将对应子模块从模组中卸载
///
/// 2、特定错误进行重试或只通知
pub(super) async fn manager_submodule(
    mut module_operate_receiver: UnboundedReceiver<ModuleOperate>,
) -> Result<()> {
    info!("Start Receive ModuleOperate");
    while let Some(module_operate) = module_operate_receiver.recv().await {
        match module_operate.operate_type {
            OperateType::REGISTER => match register_submodule(module_operate).await {
                Ok(register_submodule_name) => {
                    info!("Register Submodule {:?} success", register_submodule_name);
                }
                Err(e) => {
                    error!("Register Submodule Error: {}", e)
                }
            },
            OperateType::OFFLINE => match offline_submodule(module_operate).await {
                Ok(offline_submodule_name) => {
                    info!("Offline Submodule {:?} success", offline_submodule_name);
                }
                Err(e) => {
                    error!("Offline Submodule Error: {}", e)
                }
            },
            OperateType::HEARTBEAT => {
                // 此方法内部抛出的错误无法忽略
                update_submodule_heartbeat(module_operate).await?;
            }
            OperateType::UPDATE => match update_submodule(module_operate).await {
                Ok(update_submodule_name) => {
                    info!("Update Submodule {:?} success", update_submodule_name);
                }
                Err(e) => {
                    error!("Update Submodule Error: {}", e)
                }
            },
        }
    }
    Ok(())
}

/// 更新子模块指令设置，如果要更新连接设置应该先离线然后注册，而不是发送更新操作
async fn update_submodule(module_operate: ModuleOperate) -> Result<String> {
    info!(
        "Update Submodule {:?} Default Instruct",
        &module_operate.name
    );
    let mut new_instruct = HashMap::<String, Vec<f32>>::new();
    let mut update_instruct_map = HashMap::<String, String>::new();
    let mut remove_point_ids = Vec::<String>::new();
    // 确认子模块指令新增的指令，获取去除指令的point_id，没有变化的指令直接获取point_id
    {
        let mut locked_submodule_map = SUBMODULE_MAP.lock().await;
        if let Some(submodule) = locked_submodule_map.get_mut(module_operate.name.as_str()) {
            let mut retain_instruct = Vec::<String>::new();
            for instruct in module_operate.default_instruct.iter() {
                match submodule.default_instruct_map.get(instruct.as_str()) {
                    None => {
                        new_instruct.insert(instruct.to_string(), Vec::new());
                    }
                    Some(_) => {
                        retain_instruct.push(instruct.to_string());
                    }
                }
            }
            if submodule.default_instruct_map.len() > retain_instruct.len() {
                for instruct in retain_instruct {
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
    loop {
        if let Ok(mut lock_encoder) = INSTRUCT_ENCODER.try_lock() {
            for (instruct, _) in new_instruct.clone() {
                new_instruct.insert(
                    instruct.to_string(),
                    lock_encoder.encode(instruct.to_string())?,
                );
            }
            break;
        }
    }
    // 将新增的指令分配的point_id存入，然后在qdrant上移除需要删除指令对应的点，最后插入新增指令的点
    {
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
        let locked_instruct_manager = INSTRUCT_MANAGER.lock().await;
        locked_instruct_manager
            .append_points(module_operate.name.to_string(), insert_points)
            .await?;
        locked_instruct_manager
            .remove_points(remove_point_ids)
            .await?;
    }
    // 最后替换default_instruct_map
    {
        let mut locked_submodule_map = SUBMODULE_MAP.lock().await;
        if let Some(submodule) = locked_submodule_map.get_mut(module_operate.name.as_str()) {
            submodule.default_instruct_map = update_instruct_map;
        }
    }
    Ok(module_operate.name.to_string())
}

/// 更新子模块心跳时间
async fn update_submodule_heartbeat(module_operate: ModuleOperate) -> Result<()> {
    debug!("Submodule {:?} Heartbeat", &module_operate.name);
    let mut locked_submodule_map = SUBMODULE_MAP.lock().await;
    if let Some(submodule) = locked_submodule_map.get_mut(module_operate.name.as_str()) {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        submodule.heartbeat_time = timestamp;
    }
    Ok(())
}

/// 离线子模块处理
async fn offline_submodule(module_operate: ModuleOperate) -> Result<String> {
    info!("Offline Submodule {:?}", &module_operate.name);
    let mut point_ids = Vec::<String>::new();
    {
        let mut locked_submodule_map = SUBMODULE_MAP.lock().await;
        if let Some(submodule) = locked_submodule_map.get(module_operate.name.as_str()) {
            for (_, point_id) in &submodule.default_instruct_map {
                point_ids.push(point_id.to_string());
            }
            locked_submodule_map.remove(module_operate.name.as_str());
        }
    }
    INSTRUCT_MANAGER
        .lock()
        .await
        .remove_points(point_ids)
        .await?;
    Ok(module_operate.name.to_string())
}

/// 注册子模块处理
async fn register_submodule(module_operate: ModuleOperate) -> Result<String> {
    info!("start register model：{:?}", &module_operate.name);
    let register_submodule_name = module_operate.name.to_string();
    let mut submodule = Submodule::create_by_operate(module_operate).await?;
    let mut points = Vec::<PointPayload>::new();
    // 此处操作需要同时获取两个锁，所以只有都获取时才进行操作，否则释放锁等待下次获取锁
    loop {
        let mut locked_submodule_map = SUBMODULE_MAP.lock().await;
        match INSTRUCT_ENCODER.try_lock() {
            Ok(mut locked_instruct_encoder) => {
                debug!("Lock (submodule_map, instruct_encoder) Success");
                if let Some(_) = locked_submodule_map.get(submodule.name.as_str()) {
                    return Err(anyhow!(
                        "The Current Submodule {:?} Is Registered",
                        &submodule.name
                    ));
                }
                for (instruct, _) in submodule.default_instruct_map.clone() {
                    let encode_result = locked_instruct_encoder.encode(instruct.to_string())?;
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
                locked_submodule_map.insert(submodule.name.to_string(), submodule);
                break;
            }
            Err(_) => {
                drop(locked_submodule_map);
                continue;
            }
        }
    }
    INSTRUCT_MANAGER
        .lock()
        .await
        .append_points(register_submodule_name.to_string(), points)
        .await?;
    Ok(register_submodule_name)
}
