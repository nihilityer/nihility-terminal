use std::collections::HashMap;
use std::ops::Index;
use std::string::ToString;
use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use nihility_common::manipulate::ManipulateType;
use qdrant_client::prelude::point_id::PointIdOptions;
use qdrant_client::prelude::{
    Distance, Payload, PointStruct, QdrantClient, QdrantClientConfig, Value,
};
use qdrant_client::qdrant::points_selector::PointsSelectorOneOf;
use qdrant_client::qdrant::value::Kind::StringValue;
use qdrant_client::qdrant::vectors_config::Config;
use qdrant_client::qdrant::with_payload_selector::SelectorOptions::Enable;
use qdrant_client::qdrant::{
    CreateCollection, PointId, PointsIdsList, PointsSelector, ScoredPoint, SearchPoints,
    VectorParams, VectorsConfig, WithPayloadSelector,
};
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

use crate::core::encoder::Encoder;
use crate::core::module_manager::ModuleManager;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::{Module, ModuleOperate, OperateType};
use crate::AppError;

const COLLECTION_NAME: &str = "instruct";
const MODULE_NAME: &str = "module_name";
const INSTRUCT: &str = "instruct";
const CHUNK_SIZE: usize = 4;
const CONFIDENCE_THRESHOLD: f32 = 0.7;

pub struct GrpcQdrant;

#[async_trait]
impl ModuleManager for GrpcQdrant {
    async fn start(
        encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
        module_operate_receiver: Receiver<ModuleOperate>,
        instruct_receiver: Receiver<InstructEntity>,
        manipulate_receiver: Receiver<ManipulateEntity>,
    ) -> Result<(), AppError> {
        tracing::info!("GrpcQdrant start");

        let module_list = Arc::new(tokio::sync::Mutex::new(HashMap::<String, Module>::new()));

        let mut encode_size = 0;
        if let Ok(locked_encoder) = encoder.lock() {
            encode_size = locked_encoder.encode_size();
        }

        let qdrant_client = QdrantClientConfig::from_url("http://192.168.0.100:6334").build()?;

        let mut collection_created = false;
        let collections = qdrant_client.list_collections().await?;
        for collection in collections.collections {
            if collection.name.eq(COLLECTION_NAME) {
                // 暂时不判断向量配置等是否相等，等影响体验再进行优化
                collection_created = true;
                break;
            }
        }
        if !collection_created {
            qdrant_client
                .create_collection(&CreateCollection {
                    collection_name: COLLECTION_NAME.to_string(),
                    vectors_config: Some(VectorsConfig {
                        config: Some(Config::Params(VectorParams {
                            size: encode_size,
                            distance: Distance::Cosine.into(),
                            ..Default::default()
                        })),
                    }),
                    ..Default::default()
                })
                .await?;
        }

        let qdrant_client = Arc::new(tokio::sync::Mutex::new(qdrant_client));

        let instruct_encoder = encoder.clone();

        let module_feature = Self::manager_module(
            module_list.clone(),
            qdrant_client.clone(),
            encoder,
            module_operate_receiver,
        );

        let instruct_feature = Self::manager_instruct(
            module_list.clone(),
            qdrant_client.clone(),
            instruct_encoder,
            instruct_receiver,
        );

        let manipulate_feature = Self::manager_manipulate(module_list.clone(), manipulate_receiver);

        tokio::try_join!(module_feature, instruct_feature, manipulate_feature)?;

        Ok(())
    }
}

impl GrpcQdrant {
    /// 处理操作的接收和转发
    ///
    /// 1、通过操作实体转发操作
    ///
    /// 2、记录日志
    ///
    /// 3、处理特定的错误
    async fn manager_manipulate(
        module_map: Arc<tokio::sync::Mutex<HashMap<String, Module>>>,
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
        module_map: Arc<tokio::sync::Mutex<HashMap<String, Module>>>,
        qdrant_client: Arc<tokio::sync::Mutex<QdrantClient>>,
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
            let mut search_result = Vec::<ScoredPoint>::new();
            {
                let lock_qdrant_client = qdrant_client.lock().await;
                let search_req = SearchPoints {
                    collection_name: COLLECTION_NAME.to_string(),
                    vector: encoded_instruct,
                    limit: 1,
                    with_payload: Some(WithPayloadSelector {
                        selector_options: Some(Enable(true)),
                    }),
                    ..Default::default()
                };
                tracing::debug!("search instruct request: {:?}", &search_req);
                let search_resp = lock_qdrant_client.search_points(&search_req).await?;
                tracing::debug!("search instruct response: {:?}", &search_resp);
                search_result.append(search_resp.result.clone().as_mut());
            }
            if !search_result.is_empty() {
                let best_point = search_result.index(0);
                tracing::debug!("best point score is {}", best_point.score);
                if best_point.score >= CONFIDENCE_THRESHOLD {
                    let payload = best_point.clone().payload;
                    tracing::debug!("best point payload: {:?}", &payload);
                    if let (Some(name_kind), Some(instruct_kind)) =
                        (payload.get(MODULE_NAME), payload.get(INSTRUCT))
                    {
                        if let (
                            Some(StringValue(module_name)),
                            Some(StringValue(default_instruct)),
                        ) = (name_kind.clone().kind, instruct_kind.clone().kind)
                        {
                            tracing::info!("search result module_name is {:?}", &module_name);
                            tracing::debug!("default_instruct is {:?}", &default_instruct);
                            let mut modules = module_map.lock().await;
                            if let Some(module) = modules.get_mut(module_name.as_str()) {
                                tracing::debug!("get module by id result is {:?}", module.name);
                                let send_resp = module.send_instruct(instruct).await?;
                                tracing::debug!("send_instruct result: {:?}", send_resp);
                                continue;
                            }
                        }
                    }
                }
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
        module_map: Arc<tokio::sync::Mutex<HashMap<String, Module>>>,
        qdrant_client: Arc<tokio::sync::Mutex<QdrantClient>>,
        instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
        mut module_operate_receiver: Receiver<ModuleOperate>,
    ) -> Result<(), AppError> {
        tracing::debug!("module_receiver start recv");
        while let Some(module_operate) = module_operate_receiver.recv().await {
            match module_operate.operate_type {
                OperateType::REGISTER => {
                    let module = Module::create_by_operate(module_operate).await?;
                    Self::register_sub_module(
                        module_map.clone(),
                        qdrant_client.clone(),
                        instruct_encoder.clone(),
                        module,
                    )
                    .await?;
                }
                OperateType::OFFLINE => {
                    Self::offline_sub_module(
                        module_map.clone(),
                        qdrant_client.clone(),
                        module_operate,
                    )
                    .await?;
                }
                OperateType::HEARTBEAT => {}
                OperateType::UPDATE => {
                    Self::update_sub_module(
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
    async fn update_sub_module(
        module_map: Arc<tokio::sync::Mutex<HashMap<String, Module>>>,
        qdrant_client: Arc<tokio::sync::Mutex<QdrantClient>>,
        instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
        module_operate: ModuleOperate,
    ) -> Result<(), AppError> {
        let mut new_instruct = HashMap::<String, Vec<f32>>::new();
        let mut update_instruct_map = HashMap::<String, String>::new();
        let mut remove_point_ids = Vec::<PointId>::new();
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
                    remove_point_ids.push(PointId {
                        point_id_options: Some(PointIdOptions::Uuid(point_id.to_string())),
                    });
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
            let locked_qdrant_client = qdrant_client.lock().await;
            // 创建公共部分负载
            let mut payload = HashMap::<String, Value>::new();
            payload.insert(
                MODULE_NAME.to_string(),
                Value {
                    kind: Some(StringValue(module_operate.name.to_string())),
                },
            );
            let mut insert_points = Vec::<PointStruct>::new();
            for (instruct, encode_result) in new_instruct {
                let mut instruct_payload = payload.clone();
                instruct_payload.insert(
                    INSTRUCT.to_string(),
                    Value {
                        kind: Some(StringValue(instruct.to_string())),
                    },
                );
                let id = Uuid::new_v4();
                update_instruct_map.insert(instruct.to_string(), id.to_string());
                insert_points.push(PointStruct::new(
                    id.to_string(),
                    encode_result.clone(),
                    Payload::new_from_hashmap(instruct_payload),
                ));
            }
            locked_qdrant_client
                .delete_points(
                    COLLECTION_NAME,
                    &PointsSelector {
                        points_selector_one_of: Some(PointsSelectorOneOf::Points(PointsIdsList {
                            ids: remove_point_ids,
                        })),
                    },
                    None,
                )
                .await?;
            locked_qdrant_client
                .upsert_points_batch_blocking(COLLECTION_NAME, insert_points, None, CHUNK_SIZE)
                .await?;
        }
        // 最后替换default_instruct_map
        {
            let mut locked_module_map = module_map.lock().await;
            if let Some(module) = locked_module_map.get_mut(module_operate.name.as_str()) {
                module.default_instruct_map = update_instruct_map;
            }
        };
        Ok(())
    }

    /// 离线子模块处理
    async fn offline_sub_module(
        module_map: Arc<tokio::sync::Mutex<HashMap<String, Module>>>,
        qdrant_client: Arc<tokio::sync::Mutex<QdrantClient>>,
        module_operate: ModuleOperate,
    ) -> Result<(), AppError> {
        let mut point_ids = Vec::<PointId>::new();
        {
            let mut locked_module_map = module_map.lock().await;
            if let Some(module) = locked_module_map.get(module_operate.name.as_str()) {
                for (_, point_id) in &module.default_instruct_map {
                    point_ids.push(PointId {
                        point_id_options: Some(PointIdOptions::Uuid(point_id.to_string())),
                    });
                }
                locked_module_map.remove(module_operate.name.as_str());
            }
        }
        let locked_qdrant_client = qdrant_client.lock().await;
        locked_qdrant_client
            .delete_points(
                COLLECTION_NAME,
                &PointsSelector {
                    points_selector_one_of: Some(PointsSelectorOneOf::Points(PointsIdsList {
                        ids: point_ids,
                    })),
                },
                None,
            )
            .await?;
        Ok(())
    }

    /// 注册类型模型操作处理
    async fn register_sub_module(
        module_map: Arc<tokio::sync::Mutex<HashMap<String, Module>>>,
        qdrant_client: Arc<tokio::sync::Mutex<QdrantClient>>,
        instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
        mut module: Module,
    ) -> Result<(), AppError> {
        tracing::info!("start register model：{:?}", &module.name);
        let mut points = Vec::<PointStruct>::new();
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
                    // 创建公共部分负载
                    let mut payload = HashMap::<String, Value>::new();
                    payload.insert(
                        MODULE_NAME.to_string(),
                        Value {
                            kind: Some(StringValue(module.name.to_string())),
                        },
                    );
                    // 先将所有指令编码，之后统一插入，减少网络通信的损耗
                    for (instruct, _) in module.default_instruct_map.clone() {
                        let mut instruct_payload = payload.clone();
                        instruct_payload.insert(
                            INSTRUCT.to_string(),
                            Value {
                                kind: Some(StringValue(instruct.to_string())),
                            },
                        );
                        let encode_result = locked_instruct_encoder.encode(instruct.to_string())?;
                        let id = Uuid::new_v4();
                        module
                            .default_instruct_map
                            .insert(instruct.to_string(), id.to_string());
                        points.push(PointStruct::new(
                            id.to_string(),
                            encode_result,
                            Payload::new_from_hashmap(instruct_payload),
                        ));
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
        let locked_qdrant_client = qdrant_client.lock().await;
        // 目前以最新注册的模块指令为准，之后更新体验决定是否保留或继续覆盖
        locked_qdrant_client
            .upsert_points_batch_blocking(COLLECTION_NAME, points, None, CHUNK_SIZE)
            .await?;
        Ok(())
    }
}
