use std::collections::HashMap;
use std::ops::Index;
use std::string::ToString;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use qdrant_client::client::QdrantClient;
use qdrant_client::prelude::point_id::PointIdOptions;
use qdrant_client::prelude::{
    Distance, Payload, PointStruct, QdrantClientConfig, SearchPoints, Value,
};
use qdrant_client::qdrant::points_selector::PointsSelectorOneOf;
use qdrant_client::qdrant::value::Kind::StringValue;
use qdrant_client::qdrant::vectors_config::Config;
use qdrant_client::qdrant::with_payload_selector::SelectorOptions::Enable;
use qdrant_client::qdrant::{
    CreateCollection, PointId, PointsIdsList, PointsSelector, ScoredPoint, VectorParams,
    VectorsConfig, WithPayloadSelector,
};
use tracing::{debug, info};

use crate::core::instruct_manager::{InstructManager, PointPayload, ENCODE_SIZE_FIELD};

pub const QDRANT_GRPC_ADDR_FIELD: &str = "qdrant_grpc_addr";
const COLLECTION_NAME: &str = "instruct";
const MODULE_NAME: &str = "module_name";
const INSTRUCT: &str = "instruct";
const CHUNK_SIZE: usize = 4;
const CONFIDENCE_THRESHOLD: f32 = 0.7;

pub struct GrpcQdrant {
    qdrant_client: QdrantClient,
}

#[async_trait]
impl InstructManager for GrpcQdrant {
    async fn init(config: HashMap<String, String>) -> Result<Self>
    where
        Self: Sized + Send + Sync,
    {
        if config.get(ENCODE_SIZE_FIELD).is_none() {
            return Err(anyhow!(
                "Required configuration {:?} is missing",
                ENCODE_SIZE_FIELD
            ));
        }
        if config.get(QDRANT_GRPC_ADDR_FIELD).is_none() {
            return Err(anyhow!(
                "Required configuration {:?} is missing",
                QDRANT_GRPC_ADDR_FIELD
            ));
        }
        let qdrant_client =
            QdrantClientConfig::from_url(config.get(QDRANT_GRPC_ADDR_FIELD).unwrap()).build()?;
        let encode_size = config.get(ENCODE_SIZE_FIELD).unwrap().parse::<u64>()?;

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
        Ok(GrpcQdrant { qdrant_client })
    }

    async fn search(&self, encode: Vec<f32>) -> Result<String> {
        let mut search_result = Vec::<ScoredPoint>::new();
        {
            let search_req = SearchPoints {
                collection_name: COLLECTION_NAME.to_string(),
                vector: encode,
                limit: 1,
                with_payload: Some(WithPayloadSelector {
                    selector_options: Some(Enable(true)),
                }),
                ..Default::default()
            };
            debug!("search instruct request: {:?}", &search_req);
            let search_resp = self.qdrant_client.search_points(&search_req).await?;
            debug!("search instruct response: {:?}", &search_resp);
            search_result.append(search_resp.result.clone().as_mut());
        }
        if !search_result.is_empty() {
            let best_point = search_result.index(0);
            debug!("best point score is {}", best_point.score);
            if best_point.score >= CONFIDENCE_THRESHOLD {
                let payload = best_point.clone().payload;
                debug!("best point payload: {:?}", &payload);
                if let (Some(name_kind), Some(instruct_kind)) =
                    (payload.get(MODULE_NAME), payload.get(INSTRUCT))
                {
                    if let (Some(StringValue(module_name)), Some(StringValue(default_instruct))) =
                        (name_kind.clone().kind, instruct_kind.clone().kind)
                    {
                        info!("search result module_name is {:?}", &module_name);
                        debug!("default_instruct is {:?}", &default_instruct);
                        return Ok(module_name);
                    }
                }
            }
        }
        return Err(anyhow!("Cannot Search Match Submodule By This Instruct"));
    }

    async fn append_points(&self, module_name: String, points: Vec<PointPayload>) -> Result<()> {
        let mut point_structs = Vec::<PointStruct>::new();
        // 创建公共部分负载
        let mut payload = HashMap::<String, Value>::new();
        payload.insert(
            MODULE_NAME.to_string(),
            Value {
                kind: Some(StringValue(module_name.to_string())),
            },
        );
        // 先将所有指令编码，之后统一插入，减少网络通信的损耗
        for point in points {
            let mut instruct_payload = payload.clone();
            instruct_payload.insert(
                INSTRUCT.to_string(),
                Value {
                    kind: Some(StringValue(point.instruct.to_string())),
                },
            );
            point_structs.push(PointStruct::new(
                point.uuid.to_string(),
                point.encode.clone(),
                Payload::new_from_hashmap(instruct_payload),
            ));
        }
        self.qdrant_client
            .upsert_points_batch_blocking(COLLECTION_NAME, point_structs, None, CHUNK_SIZE)
            .await?;
        Ok(())
    }

    async fn remove_points(&self, points: Vec<String>) -> Result<()> {
        let mut point_ids = Vec::<PointId>::new();
        for point in points {
            point_ids.push(PointId {
                point_id_options: Some(PointIdOptions::Uuid(point.to_string())),
            })
        }
        self.qdrant_client
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
}
