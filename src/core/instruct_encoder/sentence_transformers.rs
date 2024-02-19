use std::ops::Deref;
use std::panic::catch_unwind;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ndarray::{Array1, Axis};
use ort::{inputs, CPUExecutionProvider, GraphOptimizationLevel, Session};
use tokenizers::Tokenizer;
use tracing::debug;

use crate::config::InstructEncoderConfig;
use crate::core::instruct_encoder::InstructEncoder;

pub const ENCODE_SIZE: u64 = 512;
pub const MODULE_PATH: &str = "module_path";
pub const MODULE_NAME: &str = "model_name";

pub struct SentenceTransformers {
    pub ort_session: Session,
    pub tokenizer: Tokenizer,
}

#[async_trait]
impl InstructEncoder for SentenceTransformers {
    async fn init(instruct_encoder_config: &InstructEncoderConfig) -> Result<Self>
    where
        Self: Sized + Send + Sync,
    {
        let model_path = instruct_encoder_config
            .config_map
            .get(MODULE_PATH)
            .expect("SentenceTransformers Config Field 'MODULE_PATH' Missing");
        let model_name = instruct_encoder_config
            .config_map
            .get(MODULE_NAME)
            .expect("SentenceTransformers Config Field 'MODULE_NAME' Missing");
        let onnx_model_path = format!("{}/{}/model.onnx", model_path, model_name);
        let tokenizers_config_path = format!("{}/{}/tokenizer.json", model_path, model_name);
        debug!("Use onnx_model_path: {}", &onnx_model_path);
        debug!("Use tokenizers_config_path: {}", &tokenizers_config_path);
        if let Err(_) = catch_unwind(|| {
            ort::init_from(instruct_encoder_config.ort_lib_path.to_string())
                .with_execution_providers([CPUExecutionProvider::default().build()])
                .commit()
        }) {
            return Err(anyhow!("Ort Init Error, Please Check The Config!"));
        }
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

    async fn encode(&self, input: &str) -> Result<Vec<f32>> {
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

    async fn encode_size(&self) -> u64 {
        ENCODE_SIZE
    }
}
