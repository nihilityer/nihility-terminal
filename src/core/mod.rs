use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use nihility_common::manipulate::ManipulateType;
use nihility_common::response_code::RespCode;
use tokio::spawn;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, WeakUnboundedSender};
use tokio_util::sync::CancellationToken;
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
    cancellation_token: CancellationToken,
    shutdown_sender: UnboundedSender<String>,
    module_operate_sender: WeakUnboundedSender<ModuleOperate>,
    module_operate_receiver: UnboundedReceiver<ModuleOperate>,
    instruct_receiver: UnboundedReceiver<InstructEntity>,
    manipulate_receiver: UnboundedReceiver<ManipulateEntity>,
) -> Result<()> {
    info!("Core Start Init");
    let submodule_map = Arc::new(tokio::sync::Mutex::new(HashMap::<String, Submodule>::new()));
    let encoder = encoder::encoder_builder(&core_config.encoder)?;

    if let Ok(locked_encoder) = encoder.lock() {
        let encode_size = locked_encoder.encode_size();
        core_config.module_manager.config_map.insert(
            instruct_manager::ENCODE_SIZE_FIELD.to_string(),
            encode_size.to_string(),
        );
    } else {
        return Err(anyhow!("Lock Instruct Encoder Error"));
    }

    let built_instruct_manager =
        instruct_manager::build_instruct_manager(core_config.module_manager).await?;

    let submodule_submodule_map = submodule_map.clone();
    let submodule_built_instruct_manager = built_instruct_manager.clone();
    let submodule_encoder = encoder.clone();
    let submodule_cancellation_token = cancellation_token.clone();
    let submodule_shutdown_sender = shutdown_sender.clone();
    spawn(async move {
        if let Err(e) = manager_submodule(
            submodule_submodule_map,
            submodule_built_instruct_manager,
            submodule_encoder,
            module_operate_receiver,
        )
        .await
        {
            error!("Submodule Manager Error: {}", e);
            submodule_cancellation_token.cancel();
        }
        submodule_shutdown_sender
            .send("Submodule Manager".to_string())
            .unwrap();
    });

    let heartbeat_submodule_map = submodule_map.clone();
    let heartbeat_cancellation_token = cancellation_token.clone();
    let heartbeat_shutdown_sender = shutdown_sender.clone();
    spawn(async move {
        if let Err(e) = manager_heartbeat(heartbeat_submodule_map, module_operate_sender).await {
            error!("Heartbeat Manager Error: {}", e);
            heartbeat_cancellation_token.cancel();
        }
        heartbeat_shutdown_sender
            .send("Heartbeat Manager".to_string())
            .unwrap();
    });

    let instruct_submodule_map = submodule_map.clone();
    let instruct_built_instruct_manager = built_instruct_manager.clone();
    let instruct_encoder = encoder.clone();
    let instruct_cancellation_token = cancellation_token.clone();
    let instruct_shutdown_sender = shutdown_sender.clone();
    spawn(async move {
        if let Err(e) = manager_instruct(
            instruct_submodule_map,
            instruct_built_instruct_manager,
            instruct_encoder,
            instruct_receiver,
        )
        .await
        {
            error!("Instruct Manager Error: {}", e);
            instruct_cancellation_token.cancel();
        }
        instruct_shutdown_sender
            .send("Instruct Manager".to_string())
            .unwrap();
    });

    let manipulate_submodule_map = submodule_map.clone();
    let manipulate_cancellation_token = cancellation_token.clone();
    let manipulate_shutdown_sender = shutdown_sender.clone();
    spawn(async move {
        if let Err(e) = manager_manipulate(manipulate_submodule_map, manipulate_receiver).await {
            error!("Manipulate Manager Error: {}", e);
            manipulate_cancellation_token.cancel();
        }
        manipulate_shutdown_sender
            .send("Manipulate Manager".to_string())
            .unwrap();
    });

    Ok(())
}

/// 管理子模块的心跳，当有子模块心跳过期时
///
/// 通过`module_operate_sender`发送消息将对于子模块离线
async fn manager_heartbeat(
    submodule_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    module_operate_sender: WeakUnboundedSender<ModuleOperate>,
) -> Result<()> {
    loop {
        if let None = module_operate_sender.upgrade() {
            warn!("Other module_operate_sender Is Close");
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(HEARTBEAT_TIME)).await;
        debug!("Make Sure The Submodule Heartbeat Is Normal");
        let mut locked_submodule_map = submodule_map.lock().await;
        let now_timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        for (_, submodule) in locked_submodule_map.iter_mut() {
            if now_timestamp - submodule.heartbeat_time > 2 * HEARTBEAT_TIME {
                info!("Submodule {:?} Heartbeat Exception", &submodule.name);
                module_operate_sender.upgrade().unwrap().send(
                    ModuleOperate::create_by_submodule(&submodule, OperateType::OFFLINE),
                )?;
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
    mut manipulate_receiver: UnboundedReceiver<ManipulateEntity>,
) -> Result<()> {
    info!("Start Receive Manipulate");
    while let Some(manipulate) = manipulate_receiver.recv().await {
        info!("Get Manipulate：{:?}", &manipulate);
        match manipulate.manipulate_type {
            ManipulateType::OfflineType => {
                error!("Offline Type Manipulate Cannot Forward")
            }
            _ => {}
        }
        let mut locked_module_map = module_map.lock().await;
        if let Some(module) = locked_module_map.get_mut(manipulate.use_module_name.as_str()) {
            match module.send_manipulate(manipulate).await {
                Ok(RespCode::Success) => {
                    debug!("Send Manipulate Success");
                }
                Ok(other_resp_code) => {
                    error!("Send Manipulate Fail, Resp Code: {:?}", other_resp_code);
                }
                Err(e) => {
                    error!("Send Manipulate Error: {}", e);
                }
            }
        } else {
            error!(
                "Expect Use Submodule Name {:?} Cannot Find In Register Submodule",
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
    qdrant_client: Arc<tokio::sync::Mutex<Box<dyn InstructManager + Send + Sync>>>,
    instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send + Sync>>>,
    mut instruct_receiver: UnboundedReceiver<InstructEntity>,
) -> Result<()> {
    info!("Start Receive Instruct");
    while let Some(instruct) = instruct_receiver.recv().await {
        info!("Get Instruct：{:?}", &instruct);
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
                    match module.send_instruct(instruct).await {
                        Ok(RespCode::Success) => {
                            debug!("Forward Instruct Success");
                        }
                        Ok(other_resp_code) => {
                            error!("Forward Instruct Fail, Resp Code: {:?}", other_resp_code);
                        }
                        Err(e) => {
                            error!("Forward Instruct Error: {}", e);
                        }
                    }
                    continue;
                }
            }
            Err(e) => {
                warn!("Match Instruct Handler Error: {}", e);
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
async fn manager_submodule(
    submodule_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    instruct_manager: Arc<tokio::sync::Mutex<Box<dyn InstructManager + Send + Sync>>>,
    instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send + Sync>>>,
    mut module_operate_receiver: UnboundedReceiver<ModuleOperate>,
) -> Result<()> {
    info!("Start Receive ModuleOperate");
    while let Some(module_operate) = module_operate_receiver.recv().await {
        match module_operate.operate_type {
            OperateType::REGISTER => {
                match register_submodule(
                    submodule_map.clone(),
                    instruct_manager.clone(),
                    instruct_encoder.clone(),
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
                }
            }
            OperateType::OFFLINE => {
                match offline_submodule(
                    submodule_map.clone(),
                    instruct_manager.clone(),
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
                }
            }
            OperateType::HEARTBEAT => {
                // 此方法内部抛出的错误无法忽略
                update_submodule_heartbeat(submodule_map.clone(), module_operate).await?;
            }
            OperateType::UPDATE => {
                match update_submodule(
                    submodule_map.clone(),
                    instruct_manager.clone(),
                    instruct_encoder.clone(),
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
                }
            }
        }
    }
    Ok(())
}

/// 更新子模块指令设置，如果要更新连接设置应该先离线然后注册，而不是发送更新操作
async fn update_submodule(
    submodule_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    instruct_manager: Arc<tokio::sync::Mutex<Box<dyn InstructManager + Send + Sync>>>,
    instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send + Sync>>>,
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
    {
        let mut locked_submodule_map = submodule_map.lock().await;
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
        let locked_instruct_manager = instruct_manager.lock().await;
        locked_instruct_manager
            .append_points(module_operate.name.to_string(), insert_points)
            .await?;
        locked_instruct_manager
            .remove_points(remove_point_ids)
            .await?;
    }
    // 最后替换default_instruct_map
    {
        let mut locked_submodule_map = submodule_map.lock().await;
        if let Some(submodule) = locked_submodule_map.get_mut(module_operate.name.as_str()) {
            submodule.default_instruct_map = update_instruct_map;
        }
    }
    Ok(module_operate.name.to_string())
}

/// 更新子模块心跳时间
async fn update_submodule_heartbeat(
    submodule_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    module_operate: ModuleOperate,
) -> Result<()> {
    debug!("Submodule {:?} Heartbeat", &module_operate.name);
    let mut locked_submodule_map = submodule_map.lock().await;
    if let Some(submodule) = locked_submodule_map.get_mut(module_operate.name.as_str()) {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        submodule.heartbeat_time = timestamp;
    }
    Ok(())
}

/// 离线子模块处理
async fn offline_submodule(
    submodule_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    instruct_manager: Arc<tokio::sync::Mutex<Box<dyn InstructManager + Send + Sync>>>,
    module_operate: ModuleOperate,
) -> Result<String> {
    info!("Offline Submodule {:?}", &module_operate.name);
    let mut point_ids = Vec::<String>::new();
    {
        let mut locked_submodule_map = submodule_map.lock().await;
        if let Some(submodule) = locked_submodule_map.get(module_operate.name.as_str()) {
            for (_, point_id) in &submodule.default_instruct_map {
                point_ids.push(point_id.to_string());
            }
            locked_submodule_map.remove(module_operate.name.as_str());
        }
    }
    instruct_manager
        .lock()
        .await
        .remove_points(point_ids)
        .await?;
    Ok(module_operate.name.to_string())
}

/// 注册子模块处理
async fn register_submodule(
    submodule_map: Arc<tokio::sync::Mutex<HashMap<String, Submodule>>>,
    instruct_manager: Arc<tokio::sync::Mutex<Box<dyn InstructManager + Send + Sync>>>,
    instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send + Sync>>>,
    module_operate: ModuleOperate,
) -> Result<String> {
    info!("start register model：{:?}", &module_operate.name);
    let register_submodule_name = module_operate.name.to_string();
    let mut submodule = Submodule::create_by_operate(module_operate).await?;
    let mut points = Vec::<PointPayload>::new();
    // 此处操作需要同时获取两个锁，所以只有都获取时才进行操作，否则释放锁等待下次获取锁
    loop {
        let mut locked_submodule_map = submodule_map.lock().await;
        match instruct_encoder.try_lock() {
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
    instruct_manager
        .lock()
        .await
        .append_points(register_submodule_name.to_string(), points)
        .await?;
    Ok(register_submodule_name)
}
