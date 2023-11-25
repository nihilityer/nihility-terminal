use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use nihility_common::manipulate::ManipulateType;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::try_join;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::config::CoreConfig;
use crate::core::encoder::Encoder;
use crate::core::instruct_manager::{InstructManager, PointPayload};
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::{ModuleOperate, OperateType, Submodule};

pub mod encoder;
pub mod instruct_manager;

const HEARTBEAT_TIME: u64 = 30;

pub async fn core_start(
    mut core_config: CoreConfig,
    module_operate_sender: Sender<ModuleOperate>,
    module_operate_receiver: Receiver<ModuleOperate>,
    instruct_receiver: Receiver<InstructEntity>,
    manipulate_receiver: Receiver<ManipulateEntity>,
) -> Result<()> {
    let module_map = Arc::new(tokio::sync::Mutex::new(HashMap::<String, Submodule>::new()));
    let encoder = encoder::encoder_builder(&core_config.encoder)?;

    let mut encode_size = 0;
    if let Ok(locked_encoder) = encoder.lock() {
        encode_size = locked_encoder.encode_size();
    }

    core_config.module_manager.config_map.insert(
        instruct_manager::ENCODE_SIZE_FIELD.to_string(),
        encode_size.to_string(),
    );

    let built_instruct_manager =
        instruct_manager::build_instruct_manager(core_config.module_manager).await?;

    let module_feature = manager_module(
        module_map.clone(),
        built_instruct_manager.clone(),
        encoder.clone(),
        module_operate_receiver,
    );

    let heartbeat_feature = manager_heartbeat(module_map.clone(), module_operate_sender);

    let instruct_feature = manager_instruct(
        module_map.clone(),
        built_instruct_manager.clone(),
        encoder.clone(),
        instruct_receiver,
    );

    let manipulate_feature = manager_manipulate(module_map.clone(), manipulate_receiver);

    try_join!(
        module_feature,
        heartbeat_feature,
        instruct_feature,
        manipulate_feature
    )?;
    Ok(())
}

/// 管理子模块的心跳，当有子模块心跳过期时
///
/// 通过`module_operate_sender`发送消息将对于子模块离线
async fn manager_heartbeat(
    module_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    module_operate_sender: Sender<ModuleOperate>,
) -> Result<()> {
    loop {
        tokio::time::sleep(Duration::from_secs(HEARTBEAT_TIME)).await;
        debug!("Make sure the module heartbeat is normal");
        let mut locked_module_map = module_map.lock().await;
        let now_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        for (_, module) in locked_module_map.iter_mut() {
            if now_timestamp - module.heartbeat_time > 2 * HEARTBEAT_TIME {
                info!("Submodule {:?} heartbeat exception", &module.name);
                let operate = ModuleOperate::create_by_submodule(&module, OperateType::OFFLINE)?;
                module_operate_sender.send(operate).await?;
            }
        }
    }
}

/// 处理操作的接收和转发
///
/// 1、通过操作实体转发操作
///
/// 2、记录日志
///
/// 3、处理特定的错误
async fn manager_manipulate(
    module_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    mut manipulate_receiver: Receiver<ManipulateEntity>,
) -> Result<()> {
    debug!("manipulate_receiver start recv");
    while let Some(manipulate) = manipulate_receiver.recv().await {
        info!("get manipulate：{:?}", &manipulate);
        match manipulate.manipulate_type {
            ManipulateType::OfflineType => {
                error!("Offline Type Manipulate Cannot Forward")
            }
            _ => {}
        }
        let mut locked_module_map = module_map.lock().await;
        if let Some(module) = locked_module_map.get_mut(manipulate.use_module_name.as_str()) {
            module.send_manipulate(manipulate).await?;
        } else {
            error!(
                "use module name {:?} cannot find in register sub module",
                manipulate.use_module_name.to_string()
            )
        }
    }
    Ok(())
}

/// 处理指令的接收和转发
///
/// 1、通过模块索引找到处理对应指令的模块，将指令转发
///
/// 2、记录操作到日志
///
/// 3、转发指令出现错误时选择性重试
async fn manager_instruct(
    module_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    qdrant_client: Arc<tokio::sync::Mutex<Box<dyn InstructManager + Send>>>,
    instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
    mut instruct_receiver: Receiver<InstructEntity>,
) -> Result<()> {
    debug!("instruct_receiver start recv");
    while let Some(instruct) = instruct_receiver.recv().await {
        info!("get instruct：{:?}", &instruct);
        let mut encoded_instruct: Vec<f32> = Vec::new();
        if let Ok(mut encoder) = instruct_encoder.lock() {
            encoded_instruct.append(encoder.encode(instruct.instruct.to_string())?.as_mut());
        } else {
            return Err(anyhow!("Lock Instruct Encoder Error"));
        }

        let locked_instruct_manager = qdrant_client.lock().await;
        match locked_instruct_manager.search(encoded_instruct).await {
            Ok(module_name) => {
                drop(locked_instruct_manager);

                let mut modules = module_map.lock().await;
                if let Some(module) = modules.get_mut(module_name.as_str()) {
                    let send_resp = module.send_instruct(instruct).await?;
                    debug!("send_instruct result: {:?}", send_resp);
                    continue;
                }
            }
            Err(e) => {
                warn!("{}", e.to_string());
            }
        }
    }
    Ok(())
}

/// 负责管理子模块
///
/// 1、定时获取子模块心跳，当离线时将对应子模块从模组中卸载
///
/// 2、特定错误进行重试或只通知
async fn manager_module(
    module_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    qdrant_client: Arc<tokio::sync::Mutex<Box<dyn InstructManager + Send>>>,
    instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
    mut module_operate_receiver: Receiver<ModuleOperate>,
) -> Result<()> {
    debug!("module_receiver start recv");
    while let Some(module_operate) = module_operate_receiver.recv().await {
        match module_operate.operate_type {
            OperateType::REGISTER => {
                match register_submodule(
                    module_map.clone(),
                    qdrant_client.clone(),
                    instruct_encoder.clone(),
                    module_operate,
                )
                .await
                {
                    Ok(register_submodule_name) => {
                        info!("Register Submodule {:?} success", register_submodule_name);
                    }
                    Err(e) => {
                        error!("{}", e)
                    }
                }
            }
            OperateType::OFFLINE => {
                offline_submodule(module_map.clone(), qdrant_client.clone(), module_operate)
                    .await?;
            }
            OperateType::HEARTBEAT => {
                update_submodule_heartbeat(module_map.clone(), module_operate).await?;
            }
            OperateType::UPDATE => {
                update_submodule(
                    module_map.clone(),
                    qdrant_client.clone(),
                    instruct_encoder.clone(),
                    module_operate,
                )
                .await?;
            }
        }
    }
    Ok(())
}

/// 更新子模块指令设置，如果要更新连接设置应该先离线然后注册，而不是发送更新操作
async fn update_submodule(
    module_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    qdrant_client: Arc<tokio::sync::Mutex<Box<dyn InstructManager + Send>>>,
    instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
    module_operate: ModuleOperate,
) -> Result<()> {
    info!(
        "Update Submodule {:?} Default Instruct",
        &module_operate.name
    );
    let mut new_instruct = HashMap::<String, Vec<f32>>::new();
    let mut update_instruct_map = HashMap::<String, String>::new();
    let mut remove_point_ids = Vec::<String>::new();
    // 确认子模块指令新增的指令，获取去除指令的point_id，没有变化的指令直接获取point_id
    {
        let mut locked_module_map = module_map.lock().await;
        if let Some(module) = locked_module_map.get_mut(module_operate.name.as_str()) {
            let mut retain_instruct = Vec::<String>::new();
            for instruct in module_operate.default_instruct.iter() {
                match module.default_instruct_map.get(instruct.as_str()) {
                    None => {
                        new_instruct.insert(instruct.to_string(), Vec::new());
                    }
                    Some(_) => {
                        retain_instruct.push(instruct.to_string());
                    }
                }
            }
            if module.default_instruct_map.len() > retain_instruct.len() {
                for instruct in retain_instruct {
                    if let Some(point_id) = module.default_instruct_map.get(instruct.as_str()) {
                        update_instruct_map.insert(instruct.to_string(), point_id.to_string());
                        module.default_instruct_map.remove(instruct.as_str());
                    }
                }
            }
            for (_, point_id) in module.default_instruct_map.iter() {
                remove_point_ids.push(point_id.to_string())
            }
        }
    }
    // 将新增指令编码
    loop {
        if let Ok(mut lock_encoder) = instruct_encoder.try_lock() {
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
        let locked_instruct_manager = qdrant_client.lock().await;
        locked_instruct_manager
            .append_points(module_operate.name.to_string(), insert_points)
            .await?;
        locked_instruct_manager
            .remove_points(remove_point_ids)
            .await?;
    }
    // 最后替换default_instruct_map
    {
        let mut locked_module_map = module_map.lock().await;
        if let Some(module) = locked_module_map.get_mut(module_operate.name.as_str()) {
            module.default_instruct_map = update_instruct_map;
        }
    }
    Ok(())
}

/// 更新子模块心跳时间
async fn update_submodule_heartbeat(
    module_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    module_operate: ModuleOperate,
) -> Result<()> {
    debug!("Submodule {:?} Heartbeat", &module_operate.name);
    let mut locked_module_map = module_map.lock().await;
    if let Some(module) = locked_module_map.get_mut(module_operate.name.as_str()) {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        module.heartbeat_time = timestamp;
    }
    Ok(())
}

/// 离线子模块处理
async fn offline_submodule(
    module_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    qdrant_client: Arc<tokio::sync::Mutex<Box<dyn InstructManager + Send>>>,
    module_operate: ModuleOperate,
) -> Result<()> {
    info!("Offline Submodule {:?}", &module_operate.name);
    let mut point_ids = Vec::<String>::new();
    {
        let mut locked_module_map = module_map.lock().await;
        if let Some(module) = locked_module_map.get(module_operate.name.as_str()) {
            for (_, point_id) in &module.default_instruct_map {
                point_ids.push(point_id.to_string());
            }
            locked_module_map.remove(module_operate.name.as_str());
        }
    }
    let locked_instruct_manager = qdrant_client.lock().await;
    locked_instruct_manager.remove_points(point_ids).await?;
    Ok(())
}

/// 注册子模块处理
async fn register_submodule(
    module_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    qdrant_client: Arc<tokio::sync::Mutex<Box<dyn InstructManager + Send>>>,
    instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
    module_operate: ModuleOperate,
) -> Result<String> {
    info!("start register model：{:?}", &module_operate.name);
    let register_submodule_name = module_operate.name.to_string();
    let mut submodule = Submodule::create_by_operate(module_operate).await?;
    let mut points = Vec::<PointPayload>::new();
    // 此处操作需要同时获取两个锁，所以只有都获取时才进行操作，否则释放锁等待下次获取锁
    loop {
        let mut locked_module_map = module_map.lock().await;
        match instruct_encoder.try_lock() {
            Ok(mut locked_instruct_encoder) => {
                debug!("lock locked_module_map, locked_instruct_encoder success");
                if let Some(_) = locked_module_map.get(submodule.name.as_str()) {
                    return Err(anyhow!(
                        "The current submodule {:?} is registered",
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
                locked_module_map.insert(submodule.name.to_string(), submodule);
                break;
            }
            Err(_) => {
                drop(locked_module_map);
                continue;
            }
        }
    }
    let locked_instruct_manager = qdrant_client.lock().await;
    locked_instruct_manager
        .append_points(register_submodule_name.to_string(), points)
        .await?;
    Ok(register_submodule_name)
}
