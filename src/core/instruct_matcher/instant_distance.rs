use anyhow::{anyhow, Result};
use async_trait::async_trait;
use instant_distance::{Builder, HnswMap, Point, Search};

use crate::config::InstructMatcherConfig;
use crate::core::instruct_matcher::{InstructMatcher, PointPayload};

impl Point for PointPayload {
    fn distance(&self, other: &Self) -> f32 {
        let mut pow_sum = 0.0f32;
        for i in 0..self.encode.len() {
            pow_sum += (self.encode[i] - other.encode[i]) * (self.encode[i] - other.encode[i]);
        }
        pow_sum.sqrt()
    }
}

pub struct InstantDistance {
    hnsw_map: HnswMap<PointPayload, bool>,
}

#[async_trait]
impl InstructMatcher for InstantDistance {
    async fn init(_instruct_matcher_config: &InstructMatcherConfig) -> Result<Self>
    where
        Self: Sized + Send + Sync,
    {
        Ok(InstantDistance {
            hnsw_map: Builder::default().build(vec![], vec![]),
        })
    }

    async fn search(&self, point: Vec<f32>) -> Result<String> {
        match self
            .hnsw_map
            .search(
                &PointPayload {
                    encode: point,
                    ..Default::default()
                },
                &mut Search::default(),
            )
            .next()
        {
            None => Err(anyhow!("Not Search Result")),
            Some(item) => Ok(item.point.submodule_id.clone()),
        }
    }

    async fn append_points(&mut self, mut points: Vec<PointPayload>) -> Result<()> {
        points.append(
            &mut self
                .hnsw_map
                .iter()
                .map(|(_, point)| point.clone())
                .collect(),
        );
        let len = points.len();
        self.hnsw_map = Builder::default().build(points, vec![false; len]);
        Ok(())
    }

    async fn remove_points(&mut self, points: Vec<PointPayload>) -> Result<()> {
        let tmp: Vec<PointPayload> = self
            .hnsw_map
            .iter()
            .map(|(_, point)| point.clone())
            .filter(|x| !points.contains(&x))
            .collect();
        let len = tmp.len();
        self.hnsw_map = Builder::default().build(tmp, vec![false; len]);
        Ok(())
    }
}
