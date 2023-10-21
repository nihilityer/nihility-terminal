use std::ops::Deref;

use ndarray::{Array1, Axis, CowArray};
use ort::{Environment, ExecutionProvider, GraphOptimizationLevel, Session, SessionBuilder, Value};
use ort::tensor::OrtOwnedTensor;
use tokenizers::Tokenizer;

use crate::AppError;

pub struct InstructEncoder {
    pub ort_session: Session,
    pub tokenizer: Tokenizer,
}

impl InstructEncoder {
    pub fn init(
        model_path: &String,
        model_name: &String
    ) -> Result<InstructEncoder, AppError> {
        let onnx_model_path = format!("{}/{}/model.onnx", model_path, model_name);
        let tokenizers_config_path = format!("{}/{}/tokenizer.json", model_path, model_name);
        tracing::debug!("onnx_model_path:{}", &onnx_model_path);
        tracing::debug!("tokenizers_config_path:{}", &tokenizers_config_path);
        let environment = Environment::builder()
            .with_name("encoder")
            .with_execution_providers([ExecutionProvider::CPU(Default::default())])
            .build()?
            .into_arc();
        let session = SessionBuilder::new(&environment)?
            .with_optimization_level(GraphOptimizationLevel::Level1)?
            .with_model_from_file(onnx_model_path)?;

        let tokenizer = Tokenizer::from_file(tokenizers_config_path).unwrap();

        Ok(InstructEncoder {
            ort_session: session,
            tokenizer,
        })
    }

    pub fn encode(&mut self, input: String) -> Result<Vec<f32>, AppError> {
        let encoding = self.tokenizer.encode(input, false).unwrap();
        tracing::debug!("encoding: {:?}", &encoding);

        let ids = encoding
            .get_ids()
            .iter()
            .map(|i| *i as i64)
            .collect::<Vec<_>>();
        let mut ids = CowArray::from(Array1::from_iter(ids.iter().cloned()));
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
        let mut type_ids = CowArray::from(Array1::from_iter(type_ids.iter().cloned()));
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
        let mut attention_mask = CowArray::from(Array1::from_iter(attention_mask.iter().cloned()));
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
        tracing::debug!("onnx inputs: {:?}", &inputs);
        let outputs: Vec<Value> = self.ort_session.run(inputs)?;
        let generated_tokens: OrtOwnedTensor<f32, _> = outputs[0].try_extract()?;

        if let Some(result) = generated_tokens.view().deref().to_slice() {
            let result = result.iter().cloned().into_iter().collect::<Vec<f32>>();
            return Ok(result);
        } else {
            return Err(AppError::ModuleManagerError("encode error".to_string()));
        }
    }
}