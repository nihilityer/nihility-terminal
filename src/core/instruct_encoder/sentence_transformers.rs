use std::ops::Deref;

use anyhow::{anyhow, Result};
use ndarray::{Array1, Axis};
use ort::{inputs, CPUExecutionProvider, GraphOptimizationLevel, Session};
use tokenizers::Tokenizer;
use tracing::debug;

use crate::core::instruct_encoder::InstructEncoder;

const ENCODE_SIZE: u64 = 512;

pub struct SentenceTransformers {
    pub ort_session: Session,
    pub tokenizer: Tokenizer,
}

impl InstructEncoder for SentenceTransformers {
    fn init(model_path: String, model_name: String) -> Result<Self>
    where
        Self: Sized + Send + Sync,
    {
        let onnx_model_path = format!("{}/{}/model.onnx", model_path, model_name);
        let tokenizers_config_path = format!("{}/{}/tokenizer.json", model_path, model_name);
        debug!("Use onnx_model_path: {}", &onnx_model_path);
        debug!("Use tokenizers_config_path: {}", &tokenizers_config_path);
        ort::init()
            .with_execution_providers([CPUExecutionProvider::default().build()])
            .commit()?;
        let session = Session::builder()?
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
        debug!("Encoding: {:?}", &encoding);

        let input_ids = Array1::from_iter(
            encoding
                .get_ids()
                .iter()
                .map(|i| *i as i64)
                .collect::<Vec<_>>(),
        )
        .insert_axis(Axis(0));

        let token_type_ids = Array1::from_iter(
            encoding
                .get_type_ids()
                .iter()
                .map(|i| *i as i64)
                .collect::<Vec<_>>(),
        )
        .insert_axis(Axis(0));

        let attention_mask = Array1::from_iter(
            encoding
                .get_attention_mask()
                .iter()
                .map(|i| *i as i64)
                .collect::<Vec<_>>(),
        )
        .insert_axis(Axis(0));

        let inputs = inputs![
            "input_ids" => input_ids,
            "token_type_ids" => token_type_ids,
            "attention_mask" => attention_mask,
        ]?;
        debug!("Onnx Inputs: {:?}", &inputs);
        let outputs = self.ort_session.run(inputs)?;
        let generated_tokens = outputs[0].extract_tensor::<f32>()?;
        let encode_result = generated_tokens.view();
        let encode_result = encode_result.deref().index_axis(Axis(0), 0);

        return if let Some(result) = encode_result.mean_axis(Axis(0)) {
            let result = result.iter().cloned().collect::<Vec<f32>>();
            debug!("Encode Result Len:{:?}", result.len());
            Ok(result)
        } else {
            Err(anyhow!("Encode Result Transform Error"))
        };
    }

    fn encode_size(&self) -> u64 {
        ENCODE_SIZE
    }
}
