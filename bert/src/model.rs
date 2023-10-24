// Copyright 2021 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{env, sync::Arc};

use anyhow::{bail, Error};
use ndarray::{Array, CowArray, IxDyn};
use ort::{
    environment::Environment,
    execution_providers::{
        ACLExecutionProviderOptions,
        CPUExecutionProviderOptions,
        CUDAExecutionProviderOptions,
        ExecutionProvider,
        TensorRTExecutionProviderOptions,
    },
    session::{Session, SessionBuilder},
    tensor::OrtOwnedTensor,
    value::Value,
    GraphOptimizationLevel,
    LoggingLevel,
};
use tokenizers::Encoding;

use crate::config::Config;

/// A Bert onnx model.
#[derive(Debug)]
pub(crate) struct Model {
    runtime: Session,
    use_type_ids: bool,
    pub(crate) embedding_size: usize,
    // we drop env last. This as been fixed upstream but not yet released.
    _env: Arc<Environment>,
}

pub(crate) struct Embedding(Value<'static>);

impl Model {
    /// Creates a model from a configuration.
    pub(crate) fn new<P>(config: &Config<P>) -> Result<Self, Error> {
        env::set_var("ORT_DYLIB_PATH", config.runtime()?);
        let environment = Environment::builder()
            .with_name("embedder")
            .with_execution_providers([
                // TODO: add onnxruntime gpu libraries to assets
                ExecutionProvider::TensorRT(TensorRTExecutionProviderOptions::default()),
                ExecutionProvider::CUDA(CUDAExecutionProviderOptions::default()),
                ExecutionProvider::ACL(ACLExecutionProviderOptions::default()),
                ExecutionProvider::CPU(CPUExecutionProviderOptions::default()),
            ])
            .with_log_level(LoggingLevel::Warning)
            .build()?
            .into_arc();
        let session = SessionBuilder::new(&environment)?
            // TODO: this is the default, we could run the optimizations once offline and then
            // always load the optimized model from disk with GraphOptimizationLevel::Disable
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_model_from_file(config.model()?)?;

        let use_type_ids = session.inputs.len() > 2;
        let Some(embedding_size) =
            session.outputs[0].dimensions[2].or_else(|| session.outputs[1].dimensions[1])
        else {
            bail!(format!(
                "embedder model '{}' has unspecified embedding size",
                config.model()?.display(),
            ));
        };

        Ok(Model {
            runtime: session,
            use_type_ids,
            embedding_size: embedding_size as usize,
            _env: environment,
        })
    }

    /// Runs embedding on the encoded sequence.
    pub(crate) fn embed(&self, encoding: &Encoding) -> Result<Embedding, Error> {
        let array_from = |slice: &[u32]| {
            CowArray::from(Array::from_shape_fn([1, slice.len()].as_slice(), |idx| {
                i64::from(slice[idx[1]])
            }))
        };
        let token_ids = array_from(encoding.get_ids());
        let attention_mask = array_from(encoding.get_attention_mask());
        let type_ids = self
            .use_type_ids
            .then(|| array_from(encoding.get_type_ids()));

        let value_from = |array| Value::from_array(self.runtime.allocator(), array);
        let token_ids = value_from(&token_ids)?;
        let attention_mask = value_from(&attention_mask)?;
        let inputs = if let Some(type_ids) = &type_ids {
            vec![token_ids, attention_mask, value_from(type_ids)?]
        } else {
            vec![token_ids, attention_mask]
        };
        let mut outputs = self.runtime.run(inputs)?;

        Ok(Embedding(outputs.swap_remove(0)))
    }
}

impl Embedding {
    pub(crate) fn extract(&self) -> Result<OrtOwnedTensor<'_, f32, IxDyn>, Error> {
        self.0.try_extract().map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use ort::tensor::TensorElementDataType;
    use xayn_test_utils::asset::{ort, smbert_mocked};

    use super::*;

    #[test]
    fn test_new() {
        let config = Config::new(smbert_mocked().unwrap(), ort().unwrap()).unwrap();
        let model = Model::new(&config).unwrap();

        assert_eq!(model.runtime.inputs.len(), 3);
        let input = &model.runtime.inputs[0];
        assert_eq!(input.dimensions, [None, None]);
        assert_eq!(input.input_type, TensorElementDataType::Int64);
        let input = &model.runtime.inputs[1];
        assert_eq!(input.dimensions, [None, None]);
        assert_eq!(input.input_type, TensorElementDataType::Int64);
        let input = &model.runtime.inputs[2];
        assert_eq!(input.dimensions, [None, None]);
        assert_eq!(input.input_type, TensorElementDataType::Int64);

        assert_eq!(model.runtime.outputs.len(), 2);
        let output = &model.runtime.outputs[0];
        assert_eq!(output.dimensions, [None, None, None]);
        assert_eq!(output.output_type, TensorElementDataType::Float32);
        let output = &model.runtime.outputs[1];
        assert_eq!(output.dimensions, [None, Some(128)]);
        assert_eq!(output.output_type, TensorElementDataType::Float32);
    }

    #[test]
    fn test_embed() {
        let token_size = 64;
        let config = Config::new(smbert_mocked().unwrap(), ort().unwrap())
            .unwrap()
            .with_token_size(token_size)
            .unwrap();
        let model = Model::new(&config).unwrap();

        let encoding = Encoding::new(
            vec![0; token_size],
            vec![0; token_size],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![1; token_size],
            Vec::new(),
            HashMap::new(),
        );
        let embedding = model.embed(&encoding).unwrap();
        assert_eq!(
            embedding.extract().unwrap().view().shape(),
            [1, token_size, model.embedding_size],
        );
    }
}
