use std::ops::Deref;

use anyhow::{anyhow, Result};
use ndarray::{Array1, Axis, CowArray};
use ort::tensor::OrtOwnedTensor;
use ort::{Environment, ExecutionProvider, GraphOptimizationLevel, Session, SessionBuilder, Value};
use tokenizers::Tokenizer;
use tracing::debug;

use crate::core::encoder::Encoder;

const ENCODE_SIZE: u64 = 512;

pub struct SentenceTransformers {
    pub ort_session: Session,
    pub tokenizer: Tokenizer,
}

impl Encoder for SentenceTransformers {
    fn init(model_path: String, model_name: String) -> Result<Self>
    where
        Self: Sized,
    {
        let onnx_model_path = format!("{}/{}/model.onnx", model_path, model_name);
        let tokenizers_config_path = format!("{}/{}/tokenizer.json", model_path, model_name);
        debug!("onnx_model_path:{}", &onnx_model_path);
        debug!("tokenizers_config_path:{}", &tokenizers_config_path);
        let environment = Environment::builder()
            .with_name("SentenceTransformers")
            .with_execution_providers([ExecutionProvider::CPU(Default::default())])
            .build()?
            .into_arc();
        let session = SessionBuilder::new(&environment)?
            .with_optimization_level(GraphOptimizationLevel::Level1)?
            .with_model_from_file(onnx_model_path)?;

        let tokenizer = Tokenizer::from_file(tokenizers_config_path).unwrap();

        let encoder = SentenceTransformers {
            ort_session: session,
            tokenizer,
        };
        Ok(encoder)
    }

    fn encode(&mut self, input: String) -> Result<Vec<f32>> {
        let encoding = self.tokenizer.encode(input, false).unwrap();
        debug!("encoding: {:?}", &encoding);

        let ids = encoding
            .get_ids()
            .iter()
            .map(|i| *i as i64)
            .collect::<Vec<_>>();
        let ids = CowArray::from(Array1::from_iter(ids.iter().cloned()));
        let n_ids = ids.shape()[0];
        let input_ids = ids
            .clone()
            .insert_axis(Axis(0))
            .into_shape((1, n_ids))
            .unwrap()
            .into_dyn();

        let type_ids = encoding
            .get_type_ids()
            .iter()
            .map(|i| *i as i64)
            .collect::<Vec<_>>();
        let type_ids = CowArray::from(Array1::from_iter(type_ids.iter().cloned()));
        let n_type_ids = type_ids.shape()[0];
        let token_type_ids = type_ids
            .clone()
            .insert_axis(Axis(0))
            .into_shape((1, n_type_ids))
            .unwrap()
            .into_dyn();

        let attention_mask = encoding
            .get_attention_mask()
            .iter()
            .map(|i| *i as i64)
            .collect::<Vec<_>>();
        let attention_mask = CowArray::from(Array1::from_iter(attention_mask.iter().cloned()));
        let n_attention_mask = attention_mask.shape()[0];
        let attention_mask = attention_mask
            .clone()
            .insert_axis(Axis(0))
            .into_shape((1, n_attention_mask))
            .unwrap()
            .into_dyn();

        let inputs = vec![
            Value::from_array(self.ort_session.allocator(), &input_ids)?,
            Value::from_array(self.ort_session.allocator(), &token_type_ids)?,
            Value::from_array(self.ort_session.allocator(), &attention_mask)?,
        ];
        debug!("onnx inputs: {:?}", &inputs);
        let outputs: Vec<Value> = self.ort_session.run(inputs)?;
        let generated_tokens: OrtOwnedTensor<f32, _> = outputs[0].try_extract()?;
        let encode_result = generated_tokens.view();
        let encode_result = encode_result.deref().index_axis(Axis(0), 0);

        if let Some(result) = encode_result.mean_axis(Axis(0)) {
            let result = result.iter().cloned().into_iter().collect::<Vec<f32>>();
            debug!("encode result len:{:?}", result.len());
            return Ok(result);
        } else {
            return Err(anyhow!("Encode Result Transform Error"));
        }
    }

    fn encode_size(&self) -> u64 {
        return ENCODE_SIZE.clone();
    }
}
