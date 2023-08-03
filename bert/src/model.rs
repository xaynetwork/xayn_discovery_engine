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

use std::{fs::File, io::BufReader, path::PathBuf};

use anyhow::bail;
use derive_more::{Deref, From};
use ndarray::CowArray;
use ort::{
    environment::Environment,
    session::{Session, SessionBuilder},
    value::Value,
};
use serde::{Deserialize, Serialize};
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

use crate::{config::Config, tokenizer::Encoding};

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

use anyhow::Result;

impl<P> Config<P> {
    fn extract_facts(
        &self,
        io: &'static str,
        mut model: InferenceModel,
        with_io_fact: impl Fn(InferenceModel, usize, InferenceFact) -> Result<InferenceModel>,
    ) -> Result<InferenceModel> {
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RuntimeKind {
    Tract,
    Ort,
}

#[derive(Debug)]
enum Runtime {
    Tract(TypedRunnableModel<TypedModel>),
    Ort(Session),
}

impl Runtime {
    pub(crate) fn new<P>(model: &PathBuf, config: &Config<P>) -> Result<Self> {
        match config.runtime_kind {
            RuntimeKind::Tract => Self::tract(model, config),
            RuntimeKind::Ort => Self::ort(model),
        }
    }

    fn tract<P>(model: &PathBuf, config: &Config<P>) -> Result<Self> {
        let mut model = BufReader::new(File::open(model)?);
        let model = tract_onnx::onnx().model_for_read(&mut model)?;
        let model = config.extract_facts("input", model, InferenceModel::with_input_fact)?;
        let model = config.extract_facts("output", model, InferenceModel::with_output_fact)?;
        let model = model.into_optimized()?.into_runnable()?;

        Ok(Self::Tract(model))
    }

    fn ort(model: &PathBuf) -> Result<Self> {
        let environment = Environment::builder()
            .with_name("embedder")
            .build()?
            .into_arc();
        let session = SessionBuilder::new(&environment)?.with_model_from_file(model)?;

        Ok(Self::Ort(session))
    }

    pub(crate) fn predict(&self, encoding: Encoding) -> Result<Prediction> {
        match self {
            Self::Tract(runtime) => Self::tract_predict(runtime, encoding),
            Self::Ort(runtime) => Self::ort_predict(runtime, encoding),
        }
    }

    fn tract_predict(
        model: &TypedRunnableModel<TypedModel>,
        encoding: Encoding,
    ) -> Result<Prediction> {
        let inputs = encoding.into();
        let mut outputs = model.run(inputs)?;

        Ok(outputs.swap_remove(0).into())
    }

    fn ort_predict(session: &Session, encoding: Encoding) -> Result<Prediction> {
        let token_ids = CowArray::from(encoding.token_ids.into_dyn());
        let attention_mask = CowArray::from(encoding.attention_mask.into_dyn());
        let type_ids = CowArray::from(encoding.type_ids.unwrap().into_dyn());

        let inputs = vec![
            Value::from_array(session.allocator(), &token_ids)?,
            Value::from_array(session.allocator(), &attention_mask)?,
            Value::from_array(session.allocator(), &type_ids)?,
        ];

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
    pub(crate) fn new<P>(config: &Config<P>) -> Result<Self> {
        let model = config.dir.join("model.onnx");
        if !model.exists() {
            bail!("embedder model '{}' doesn't exist", model.display());
        }

        let runtime = Runtime::new(&model, config)?;

        Ok(Model {
            runtime,
            token_size: config.token_size,
            embedding_size: config.extract("model.output.0.shape.2")?,
        })
    }

    /// Runs prediction on the encoded sequence.
    pub(crate) fn predict(&self, encoding: Encoding) -> Result<Prediction> {
        self.runtime.predict(encoding)
    }
}

#[cfg(test)]
mod tests {
    use std::unreachable;

    use ndarray::{Array, Array2, Dimension};
    use tract_onnx::prelude::{DatumType, IntoArcTensor};
    use xayn_test_utils::asset::smbert_mocked;

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
    fn test_new() {
        let config = Config::new(smbert_mocked().unwrap())
            .unwrap()
            .with_runtime(RuntimeKind::Tract)
            .with_token_size(64)
            .unwrap();

        let model = Model::new(&config).unwrap();
        let Runtime::Tract(tract_model) = model.runtime else { unreachable!()};
        let tract_model = tract_model.model();

        assert_eq!(tract_model.input_outlets().unwrap().len(), 3);
        let fact = tract_model.input_fact(0).unwrap();
        assert_eq!(fact.shape.as_concrete().unwrap(), [1, model.token_size]);
        assert_eq!(fact.datum_type, DatumType::I64);
        let fact = tract_model.input_fact(1).unwrap();
        assert_eq!(fact.shape.as_concrete().unwrap(), [1, model.token_size]);
        assert_eq!(fact.datum_type, DatumType::I64);
        let fact = tract_model.input_fact(2).unwrap();
        assert_eq!(fact.shape.as_concrete().unwrap(), [1, model.token_size]);
        assert_eq!(fact.datum_type, DatumType::I64);

        assert_eq!(tract_model.output_outlets().unwrap().len(), 2);
        let fact = tract_model.output_fact(0).unwrap();
        assert_eq!(
            fact.shape.as_concrete().unwrap(),
            [1, model.token_size, model.embedding_size],
        );
        assert_eq!(fact.datum_type, DatumType::F32);
        let fact = tract_model.output_fact(1).unwrap();
        assert_eq!(fact.shape.as_concrete().unwrap(), [1, model.embedding_size]);
        assert_eq!(fact.datum_type, DatumType::F32);
    }

    #[test]
    fn test_predict() {
        let shape = (1, 64);
        let config = Config::new(smbert_mocked().unwrap())
            .unwrap()
            .with_token_size(shape.1)
            .unwrap();
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
