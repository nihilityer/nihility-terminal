use std::string::ToString;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use qdrant_client::prelude::{Distance, QdrantClient, QdrantClientConfig};
use qdrant_client::qdrant::{CreateCollection, VectorParams, VectorsConfig};
use qdrant_client::qdrant::vectors_config::Config;
use tokio::sync::mpsc::Receiver;

use crate::AppError;
use crate::core::encoder::Encoder;
use crate::core::module_manager::ModuleManager;
use crate::entity::instruct::InstructEntity;
use crate::entity::manipulate::ManipulateEntity;
use crate::entity::module::Module;

const COLLECTION_NAME: &str = "instruct";

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
        let mut encode_size = 0;
        if let Ok(locked_encoder) = encoder.lock() {
            encode_size = locked_encoder.encode_size();
        }

        let qdrant_client = QdrantClientConfig::from_url("http://192.168.0.100:6334").build()?;

        let collections = qdrant_client.list_collections().await?;

        let mut collection_created = false;
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

        let module_qdrant_client = Arc::new(Mutex::new(qdrant_client));
        let instruct_qdrant_client = module_qdrant_client.clone();
        let manipulate_qdrant_client = module_qdrant_client.clone();

        let instruct_encoder = encoder.clone();

        let module_feature = Self::manager_module(module_qdrant_client, module_receiver, encoder);

        let instruct_feature =
            Self::manager_instruct(instruct_qdrant_client, instruct_receiver, instruct_encoder);

        let manipulate_feature =
            Self::manager_manipulate(manipulate_qdrant_client, manipulate_receiver);

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
        qdrant_client: Arc<Mutex<QdrantClient>>,
        mut manipulate_receiver: Receiver<ManipulateEntity>,
    ) -> Result<(), AppError> {
        tracing::debug!("manipulate_receiver start recv");
        while let Some(manipulate) = manipulate_receiver.recv().await {
            tracing::info!("get manipulate：{:?}", manipulate);
            if let Ok(_) = qdrant_client.lock() {
                tracing::debug!("get qdrant_client success");
            } else {
                return Err(AppError::ModuleManagerError(
                    "Failed to obtain qdrant_client lock".to_string(),
                ));
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
        qdrant_client: Arc<Mutex<QdrantClient>>,
        mut instruct_receiver: Receiver<InstructEntity>,
        instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
    ) -> Result<(), AppError> {
        tracing::debug!("instruct_receiver start recv");
        while let Some(instruct) = instruct_receiver.recv().await {
            tracing::info!("from mpsc receiver get instruct：{:?}", &instruct);
            for message in instruct.message {
                if let Ok(mut encoder) = instruct_encoder.lock() {
                    let v1 = encoder.encode(message)?;
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
        qdrant_client: Arc<Mutex<QdrantClient>>,
        mut module_receiver: Receiver<Module>,
        instruct_encoder: Arc<Mutex<Box<dyn Encoder + Send>>>,
    ) -> Result<(), AppError> {
        tracing::debug!("module_receiver start recv");
        while let Some(module) = module_receiver.recv().await {
            tracing::info!("register model：{:?}", &module.name);
            if let Ok(_) = qdrant_client.lock() {
                tracing::debug!("get qdrant_client success");
            } else {
                return Err(AppError::ModuleManagerError(
                    "Failed to obtain modules lock".to_string(),
                ));
            }
        }
        Ok(())
    }
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
