use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nihility_common::manipulate::ManipulateType;
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

use crate::config::CoreConfig;
use crate::core::encoder::Encoder;
use crate::core::instruct_manager::{InstructManager, PointPayload};
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::{ModuleOperate, OperateType, Submodule};
use crate::AppError;

pub mod encoder;
pub mod instruct_manager;

const HEARTBEAT_TIME: u64 = 30;

pub async fn core_start(
    mut core_config: CoreConfig,
    module_operate_sender: Sender<ModuleOperate>,
    module_operate_receiver: Receiver<ModuleOperate>,
    instruct_receiver: Receiver<InstructEntity>,
    manipulate_receiver: Receiver<ManipulateEntity>,
) -> Result<(), AppError> {
    tracing::info!(
        "Module Manager Type: {:?}",
        &core_config.module_manager.manager_type
    );
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

    let instruct_encoder = encoder.clone();

    let module_feature = manager_module(
        module_map.clone(),
        built_instruct_manager.clone(),
        encoder,
        module_operate_receiver,
    );

    let heartbeat_feature = manager_heartbeat(module_map.clone(), module_operate_sender);

    let instruct_feature = manager_instruct(
        module_map.clone(),
        built_instruct_manager.clone(),
        instruct_encoder,
        instruct_receiver,
    );

    let manipulate_feature = manager_manipulate(module_map.clone(), manipulate_receiver);

    tokio::try_join!(
        module_feature,
        heartbeat_feature,
        instruct_feature,
        manipulate_feature
    )?;
    Ok(())
}

async fn manager_heartbeat(
    module_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    module_operate_sender: Sender<ModuleOperate>,
) -> Result<(), AppError> {
    loop {
        tracing::debug!("Make sure the module heartbeat is normal");
        tokio::time::sleep(Duration::from_secs(HEARTBEAT_TIME)).await;
        let mut locked_module_map = module_map.lock().await;
        let now_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        for (_, module) in locked_module_map.iter_mut() {
            if now_timestamp - module.heartbeat_time > 2 * HEARTBEAT_TIME {
                tracing::info!("Submodule {:?} heartbeat exception", &module.name);
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
) -> Result<(), AppError> {
    tracing::debug!("manipulate_receiver start recv");
    while let Some(manipulate) = manipulate_receiver.recv().await {
        tracing::info!("get manipulate：{:?}", &manipulate);
        match manipulate.manipulate_type {
            ManipulateType::OfflineType => {
                tracing::error!("Offline Type Manipulate Cannot Forward")
            }
            _ => {}
        }
        let mut locked_module_map = module_map.lock().await;
        if let Some(module) = locked_module_map.get_mut(manipulate.use_module_name.as_str()) {
            module.send_manipulate(manipulate).await?;
        } else {
            tracing::error!(
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
) -> Result<(), AppError> {
    tracing::debug!("instruct_receiver start recv");
    while let Some(instruct) = instruct_receiver.recv().await {
        tracing::info!("from mpsc receiver get instruct：{:?}", &instruct);
        let mut encoded_instruct: Vec<f32> = Vec::new();
        if let Ok(mut encoder) = instruct_encoder.lock() {
            encoded_instruct.append(encoder.encode(instruct.instruct.to_string())?.as_mut());
        } else {
            return Err(AppError::ModuleManagerError(
                "Failed to obtain sbert lock".to_string(),
            ));
        }

        let locked_instruct_manager = qdrant_client.lock().await;
        let module_name = locked_instruct_manager.search(encoded_instruct).await?;
        drop(locked_instruct_manager);

        let mut modules = module_map.lock().await;
        if let Some(module) = modules.get_mut(module_name.as_str()) {
            tracing::debug!("get module by id result is {:?}", module.name);
            let send_resp = module.send_instruct(instruct).await?;
            tracing::debug!("send_instruct result: {:?}", send_resp);
            continue;
        }
    }
    Ok(())
}

/// 负责管理子模块
///
/// 1、定时获取子模块心跳，当离线时将对应子模块从模组中卸载
///
/// 2、后续需要实现注册子模块的构建索引
///
/// 3、特定错误进行重试或只通知
async fn manager_module(
    module_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    qdrant_client: Arc<tokio::sync::Mutex<Box<dyn InstructManager + Send>>>,
    instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
    mut module_operate_receiver: Receiver<ModuleOperate>,
) -> Result<(), AppError> {
    tracing::debug!("module_receiver start recv");
    while let Some(module_operate) = module_operate_receiver.recv().await {
        match module_operate.operate_type {
            OperateType::REGISTER => {
                let module = Submodule::create_by_operate(module_operate).await?;
                register_submodule(
                    module_map.clone(),
                    qdrant_client.clone(),
                    instruct_encoder.clone(),
                    module,
                )
                .await?;
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
) -> Result<(), AppError> {
    tracing::info!(
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
) -> Result<(), AppError> {
    tracing::debug!("Submodule {:?} Heartbeat", &module_operate.name);
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
) -> Result<(), AppError> {
    tracing::info!("Offline Submodule {:?}", &module_operate.name);
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
    mut module: Submodule,
) -> Result<(), AppError> {
    tracing::info!("start register model：{:?}", &module.name);
    let tmp_module_name = module.name.to_string();
    let mut points = Vec::<PointPayload>::new();
    // 此处操作需要同时获取两个锁，所以只有都获取时才进行操作，否则释放锁等待下次获取锁
    loop {
        let mut locked_module_map = module_map.lock().await;
        match instruct_encoder.try_lock() {
            Ok(mut locked_instruct_encoder) => {
                tracing::debug!("lock locked_module_map, locked_instruct_encoder success");
                if let Some(_) = locked_module_map.get(module.name.as_str()) {
                    tracing::error!("The current submodule {:?} is registered", &module.name);
                    return Err(AppError::RegisterError);
                }
                for (instruct, _) in module.default_instruct_map.clone() {
                    let encode_result = locked_instruct_encoder.encode(instruct.to_string())?;
                    let id = Uuid::new_v4();
                    module
                        .default_instruct_map
                        .insert(instruct.to_string(), id.to_string());
                    points.push(PointPayload {
                        encode: encode_result.clone(),
                        instruct: instruct.to_string(),
                        uuid: id.to_string(),
                    });
                }
                locked_module_map.insert(module.name.to_string(), module);
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
        .append_points(tmp_module_name, points)
        .await?;
    Ok(())
}
