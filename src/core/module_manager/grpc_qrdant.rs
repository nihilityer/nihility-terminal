use std::collections::HashMap;
use std::string::ToString;
use std::sync::{Mutex, MutexGuard};
use std::sync::Arc;

use async_trait::async_trait;
use qdrant_client::prelude::{
    Distance, Payload, PointStruct, QdrantClient, QdrantClientConfig, Value,
};
use qdrant_client::qdrant::{CreateCollection, VectorParams, VectorsConfig};
use qdrant_client::qdrant::value::Kind::{IntegerValue, StringValue};
use qdrant_client::qdrant::vectors_config::Config;
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

use crate::AppError;
use crate::core::encoder::Encoder;
use crate::core::module_manager::ModuleManager;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::Module;

const COLLECTION_NAME: &str = "instruct";
const CHUNK_SIZE: usize = 4;

pub struct GrpcQdrant;

#[async_trait]
impl ModuleManager for GrpcQdrant {
    async fn start(
        encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
        module_receiver: Receiver<Module>,
        instruct_receiver: Receiver<InstructEntity>,
        manipulate_receiver: Receiver<ManipulateEntity>,
    ) -> Result<(), AppError> {
        tracing::debug!("ModuleManager start!");

        let module_list = Arc::new(Mutex::new(Vec::<Module>::new()));

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
            module_receiver,
        );

        let instruct_feature = Self::manager_instruct(
            module_list.clone(),
            qdrant_client.clone(),
            instruct_encoder,
            instruct_receiver,
        );

        let manipulate_feature = Self::manager_manipulate(
            module_list.clone(),
            qdrant_client.clone(),
            manipulate_receiver,
        );

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
        module_list: Arc<Mutex<Vec<Module>>>,
        qdrant_client: Arc<tokio::sync::Mutex<QdrantClient>>,
        mut manipulate_receiver: Receiver<ManipulateEntity>,
    ) -> Result<(), AppError> {
        tracing::debug!("manipulate_receiver start recv");
        while let Some(manipulate) = manipulate_receiver.recv().await {
            tracing::info!("get manipulate：{:?}", manipulate);
            let _ = qdrant_client.lock().await;
            tracing::debug!("get qdrant_client success");
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
        module_list: Arc<Mutex<Vec<Module>>>,
        qdrant_client: Arc<tokio::sync::Mutex<QdrantClient>>,
        instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
        mut instruct_receiver: Receiver<InstructEntity>,
    ) -> Result<(), AppError> {
        tracing::debug!("instruct_receiver start recv");
        while let Some(instruct) = instruct_receiver.recv().await {
            tracing::info!("from mpsc receiver get instruct：{:?}", &instruct);
            if let Ok(mut encoder) = instruct_encoder.lock() {
                let v1 = encoder.encode(instruct.instruct.to_string())?;
                let v2 = encoder.encode("说，你是狗".to_string())?;
                let v3 = encoder.encode("说你是猪".to_string())?;
                tracing::info!("cosine_similarity:{}", cosine_similarity(&v1, &v2));
                tracing::info!("cosine_similarity:{}", cosine_similarity(&v1, &v3));
            } else {
                return Err(AppError::ModuleManagerError(
                    "Failed to obtain sbert lock".to_string(),
                ));
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
        module_list: Arc<Mutex<Vec<Module>>>,
        qdrant_client: Arc<tokio::sync::Mutex<QdrantClient>>,
        instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
        mut module_receiver: Receiver<Module>,
    ) -> Result<(), AppError> {
        tracing::debug!("module_receiver start recv");
        while let Some(module) = module_receiver.recv().await {
            tracing::info!("start register model：{:?}", &module.name);
            let mut points = Vec::<PointStruct>::new();
            {
                let (mut locked_module_list, mut locked_instruct_encoder) =
                    try_lock_resource(&module_list, &instruct_encoder)?;
                // 获取当前模块在数组中的索引值，方便取值
                let module_id = locked_module_list.len() as i64;
                // 创建公共部分负载
                let mut payload = HashMap::<String, Value>::new();
                payload.insert(
                    "module_id".to_string(),
                    Value {
                        kind: Some(IntegerValue(module_id)),
                    },
                );
                payload.insert(
                    "module_name".to_string(),
                    Value {
                        kind: Some(StringValue(module.name.to_string())),
                    },
                );
                // 先将所有指令编码，之后统一插入，减少网络通信的损耗
                for instruct in &module.default_instruct {
                    let mut instruct_payload = payload.clone();
                    instruct_payload.insert(
                        "instruct".to_string(),
                        Value {
                            kind: Some(StringValue(instruct.to_string())),
                        },
                    );
                    let encode_result = locked_instruct_encoder.encode(instruct.to_string())?;
                    let id = Uuid::new_v4();
                    points.push(PointStruct::new(
                        id.to_string(),
                        encode_result,
                        Payload::new_from_hashmap(instruct_payload),
                    ));
                }
                locked_module_list.push(module);
            }
            let locked_qdrant_client = qdrant_client.lock().await;
            // 目前以最新注册的模块指令为准，之后更新体验决定是否保留或继续覆盖
            locked_qdrant_client
                .upsert_points_batch_blocking(COLLECTION_NAME, points, None, CHUNK_SIZE)
                .await?;
        }
        Ok(())
    }
}

/// 尝试同时获取moduleList和qdrantClient的锁
///
/// 先阻塞获取moduleList的锁.
///
/// 获取成功后再依次尝试获取instructEncoder、qdrantClient的锁，一般不会获取失败。
///
/// 如果获取失败了，将moduleList的锁释放，再尝试获取全部锁
fn try_lock_resource<'a>(
    module_list: &'a Arc<Mutex<Vec<Module>>>,
    instruct_encoder: &'a Arc<Mutex<Box<dyn Encoder + Send>>>,
) -> Result<
    (
        MutexGuard<'a, Vec<Module>>,
        MutexGuard<'a, Box<dyn Encoder + Send>>,
    ),
    AppError,
> {
    return if let Ok(modules) = module_list.lock() {
        match instruct_encoder.try_lock() {
            Ok(encoder) => {
                tracing::debug!("try_lock_resource success");
                Ok((modules, encoder))
            }
            Err(_) => {
                drop(modules);
                try_lock_resource(module_list, instruct_encoder)
            }
        }
    } else {
        Err(AppError::ModuleManagerError(
            "Failed to obtain modules lock".to_string(),
        ))
    };
}

fn cosine_similarity(vec1: &Vec<f32>, vec2: &Vec<f32>) -> f32 {
    if vec1.len() != vec2.len() || vec1.is_empty() {
        panic!("Input vectors must have the same length and cannot be empty.");
    }

    let dot_product = vec1
        .iter()
        .zip(vec2.iter())
        .map(|(a, b)| a * b)
        .sum::<f32>();
    let magnitude1 = vec1.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude2 = vec2.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude1 == 0.0 || magnitude2 == 0.0 {
        return 0.0; // Handle division by zero
    }

    dot_product / (magnitude1 * magnitude2)
}
