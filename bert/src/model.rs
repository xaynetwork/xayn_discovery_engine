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

use std::{env, fs::File, io::BufReader};

use anyhow::Error;
use derive_more::{Deref, From};
use ndarray::CowArray;
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
    value::Value,
    GraphOptimizationLevel,
    LoggingLevel,
};
use serde::Deserialize;
use tract_onnx::prelude::{
    Framework,
    InferenceFact,
    InferenceModel,
    InferenceModelExt,
    IntoArcTensor,
    TValue,
    TypedModel,
    TypedRunnableModel,
};

use crate::{
    config::{self, Config},
    tokenizer::Encoding,
};

#[derive(Deserialize)]
enum DynDim {
    #[serde(rename = "token size")]
    TokenSize,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Dimension {
    Fixed(usize),
    Dynamic(DynDim),
}

impl<P> Config<P> {
    fn extract_facts(
        &self,
        io: &'static str,
        mut model: InferenceModel,
        with_io_fact: impl Fn(InferenceModel, usize, InferenceFact) -> Result<InferenceModel, Error>,
    ) -> Result<InferenceModel, Error> {
        let mut i = 0;
        while let Ok(datum_type) = self
            .extract::<String>(&format!("model.{io}.{i}.type"))
            .map_err(Into::into)
            .and_then(|datum_type| datum_type.parse())
        {
            let mut shape = Vec::new();
            let mut j = 0;
            while let Ok(dim) = self.extract::<Dimension>(&format!("model.{io}.{i}.shape.{j}")) {
                let dim = match dim {
                    Dimension::Fixed(dim) => dim,
                    Dimension::Dynamic(DynDim::TokenSize) => self.token_size,
                };
                shape.push(dim);
                j += 1;
            }
            model = with_io_fact(model, i, InferenceFact::dt_shape(datum_type, shape))?;
            i += 1;
        }

        Ok(model)
    }
}

/// A Bert onnx model.
#[derive(Debug)]
pub(crate) struct Model {
    runtime: Runtime,
    pub(crate) token_size: usize,
    pub(crate) embedding_size: usize,
}

#[derive(Debug)]
enum Runtime {
    Tract(TypedRunnableModel<TypedModel>),
    Ort(Session),
}

impl Runtime {
    pub(crate) fn new<P>(config: &Config<P>) -> Result<Self, Error> {
        match config.runtime {
            config::Runtime::Tract => Self::tract(config),
            config::Runtime::Ort(_) => Self::ort(config),
        }
    }

    fn tract<P>(config: &Config<P>) -> Result<Self, Error> {
        let mut model = BufReader::new(File::open(config.model()?)?);
        let model = tract_onnx::onnx().model_for_read(&mut model)?;
        let model = config.extract_facts("input", model, InferenceModel::with_input_fact)?;
        let model = config.extract_facts("output", model, InferenceModel::with_output_fact)?;
        let model = model.into_optimized()?.into_runnable()?;

        Ok(Self::Tract(model))
    }

    fn ort<P>(config: &Config<P>) -> Result<Self, Error> {
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

        Ok(Self::Ort(session))
    }

    pub(crate) fn predict(&self, encoding: Encoding) -> Result<Prediction, Error> {
        match self {
            Self::Tract(runtime) => Self::tract_predict(runtime, encoding),
            Self::Ort(runtime) => Self::ort_predict(runtime, encoding),
        }
    }

    fn tract_predict(
        model: &TypedRunnableModel<TypedModel>,
        encoding: Encoding,
    ) -> Result<Prediction, Error> {
        let inputs = encoding.into();
        let mut outputs = model.run(inputs)?;

        Ok(outputs.swap_remove(0).into())
    }

    fn ort_predict(session: &Session, encoding: Encoding) -> Result<Prediction, Error> {
        let token_ids = CowArray::from(encoding.token_ids.into_dyn());
        let attention_mask = CowArray::from(encoding.attention_mask.into_dyn());
        let type_ids = encoding
            .type_ids
            .map(|type_ids| CowArray::from(type_ids.into_dyn()));

        let token_ids = Value::from_array(session.allocator(), &token_ids)?;
        let attention_mask = Value::from_array(session.allocator(), &attention_mask)?;
        let inputs = if let Some(type_ids) = &type_ids {
            vec![
                token_ids,
                attention_mask,
                Value::from_array(session.allocator(), type_ids)?,
            ]
        } else {
            vec![token_ids, attention_mask]
        };

        let outputs = session.run(inputs)?;
        let output = outputs[0]
            .try_extract::<f32>()?
            .view()
            .to_owned()
            .into_arc_tensor();

        Ok(TValue::Const(output).into())
    }
}

/// The predicted encoding.
///
/// The prediction is of shape `(1, token_size, embedding_size)`.
#[derive(Clone, Deref, From)]
pub(crate) struct Prediction(TValue);

impl Model {
    /// Creates a model from a configuration.
    pub(crate) fn new<P>(config: &Config<P>) -> Result<Self, Error> {
        Ok(Model {
            runtime: Runtime::new(config)?,
            token_size: config.token_size,
            embedding_size: config.extract("model.output.0.shape.2")?,
        })
    }

    /// Runs prediction on the encoded sequence.
    pub(crate) fn predict(&self, encoding: Encoding) -> Result<Prediction, Error> {
        self.runtime.predict(encoding)
    }
}

#[cfg(test)]
mod tests {
    use std::unreachable;

    use ndarray::{Array, Array2, Dimension};
    use ort::tensor::TensorElementDataType;
    use tract_onnx::prelude::{DatumType, IntoArcTensor};
    use xayn_test_utils::asset::{ort, smbert_mocked};

    use super::*;

    impl<D> From<Array<f32, D>> for Prediction
    where
        D: Dimension,
    {
        fn from(array: Array<f32, D>) -> Self {
            TValue::Const(array.into_arc_tensor()).into()
        }
    }

    #[test]
    fn test_new_tract() {
        let config = Config::new(smbert_mocked().unwrap())
            .unwrap()
            .with_token_size(64)
            .unwrap()
            .with_runtime(config::Runtime::Tract);
        let Model {
            runtime,
            token_size,
            embedding_size,
        } = Model::new(&config).unwrap();
        let Runtime::Tract(model) = runtime else { unreachable!() };
        let model = model.model();

        assert_eq!(model.input_outlets().unwrap().len(), 3);
        let fact = model.input_fact(0).unwrap();
        assert_eq!(fact.shape.as_concrete().unwrap(), [1, token_size]);
        assert_eq!(fact.datum_type, DatumType::I64);
        let fact = model.input_fact(1).unwrap();
        assert_eq!(fact.shape.as_concrete().unwrap(), [1, token_size]);
        assert_eq!(fact.datum_type, DatumType::I64);
        let fact = model.input_fact(2).unwrap();
        assert_eq!(fact.shape.as_concrete().unwrap(), [1, token_size]);
        assert_eq!(fact.datum_type, DatumType::I64);

        assert_eq!(model.output_outlets().unwrap().len(), 2);
        let fact = model.output_fact(0).unwrap();
        assert_eq!(
            fact.shape.as_concrete().unwrap(),
            [1, token_size, embedding_size],
        );
        assert_eq!(fact.datum_type, DatumType::F32);
        let fact = model.output_fact(1).unwrap();
        assert_eq!(fact.shape.as_concrete().unwrap(), [1, embedding_size]);
        assert_eq!(fact.datum_type, DatumType::F32);
    }

    #[test]
    fn test_new_ort() {
        let config = Config::new(smbert_mocked().unwrap())
            .unwrap()
            .with_token_size(64)
            .unwrap()
            .with_runtime(config::Runtime::Ort(ort().unwrap()));
        let Model {
            runtime,
            embedding_size,
            ..
        } = Model::new(&config).unwrap();
        let Runtime::Ort(session) = runtime else { unreachable!() };
        #[allow(clippy::cast_possible_truncation)]
        let embedding_size = embedding_size as u32;

        assert_eq!(session.inputs.len(), 3);
        let input = &session.inputs[0];
        assert_eq!(input.dimensions, [None, None]);
        assert_eq!(input.input_type, TensorElementDataType::Int64);
        let input = &session.inputs[1];
        assert_eq!(input.dimensions, [None, None]);
        assert_eq!(input.input_type, TensorElementDataType::Int64);
        let input = &session.inputs[2];
        assert_eq!(input.dimensions, [None, None]);
        assert_eq!(input.input_type, TensorElementDataType::Int64);

        assert_eq!(session.outputs.len(), 2);
        let output = &session.outputs[0];
        assert_eq!(output.dimensions, [None, None, None]);
        assert_eq!(output.output_type, TensorElementDataType::Float32);
        let output = &session.outputs[1];
        assert_eq!(output.dimensions, [None, Some(embedding_size)]);
        assert_eq!(output.output_type, TensorElementDataType::Float32);
    }

    #[test]
    fn test_predict_tract() {
        let shape = (1, 64);
        let config = Config::new(smbert_mocked().unwrap())
            .unwrap()
            .with_token_size(shape.1)
            .unwrap()
            .with_runtime(config::Runtime::Tract);
        let model = Model::new(&config).unwrap();

        let encoding = Encoding {
            token_ids: Array2::from_elem(shape, 0),
            attention_mask: Array2::from_elem(shape, 1),
            type_ids: Some(Array2::from_elem(shape, 0)),
        };
        let prediction = model.predict(encoding).unwrap();
        assert_eq!(model.token_size, shape.1);
        assert_eq!(prediction.shape(), [shape.0, shape.1, model.embedding_size]);
    }

    #[test]
    fn test_predict_ort() {
        let shape = (1, 64);
        let config = Config::new(smbert_mocked().unwrap())
            .unwrap()
            .with_token_size(shape.1)
            .unwrap()
            .with_runtime(config::Runtime::Ort(ort().unwrap()));
        let model = Model::new(&config).unwrap();

        let encoding = Encoding {
            token_ids: Array2::from_elem(shape, 0),
            attention_mask: Array2::from_elem(shape, 1),
            type_ids: Some(Array2::from_elem(shape, 0)),
        };
        let prediction = model.predict(encoding).unwrap();
        assert_eq!(model.token_size, shape.1);
        assert_eq!(prediction.shape(), [shape.0, shape.1, model.embedding_size]);
    }
}
