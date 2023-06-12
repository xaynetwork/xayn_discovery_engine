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

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufReader, BufWriter},
};

use derive_more::{Deref, From};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tract_onnx::prelude::{
    tract_data::anyhow::anyhow,
    Framework,
    InferenceFact,
    InferenceModel,
    InferenceModelExt,
    TValue,
    TVec,
    TractError,
    TypedModel,
    TypedRunnableModel,
};

use crate::{config::Config, tokenizer::Tokenize};

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

impl<T, P> Config<T, P> {
    fn extract_facts(
        &self,
        io: &'static str,
        mut model: InferenceModel,
        with_io_fact: impl Fn(
            InferenceModel,
            usize,
            InferenceFact,
        ) -> Result<InferenceModel, TractError>,
    ) -> Result<InferenceModel, TractError> {
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
    model: TypedRunnableModel<TypedModel>,
    pub(crate) token_size: usize,
    pub(crate) embedding_size: usize,
}

/// The predicted encoding.
///
/// The prediction is of shape `(1, token_size, embedding_size)`.
#[derive(Clone, Deref, From)]
pub(crate) struct Prediction(TValue);

impl Model {
    /// Creates a model from a configuration.
    pub(crate) fn new<T, P>(config: &Config<T, P>) -> Result<Self, TractError> {
        let mut model = BufReader::new(File::open(config.dir.join("model.onnx"))?);
        let model = tract_onnx::onnx().model_for_read(&mut model)?;
        let model = config.extract_facts("input", model, InferenceModel::with_input_fact)?;
        let model = config.extract_facts("output", model, InferenceModel::with_output_fact)?;
        let model = model.into_optimized()?.into_runnable()?;

        Ok(Model {
            model,
            token_size: config.token_size,
            embedding_size: config.extract("model.output.0.shape.2")?,
        })
    }

    /// Runs prediction on the encoded sequence.
    pub(crate) fn predict(&self, inputs: TVec<TValue>) -> Result<Prediction, TractError> {
        self.model
            .run(inputs)
            .map(|mut outputs| outputs.swap_remove(0).into())
    }
}

/// A BM25 training model.
#[derive(Debug, Default)]
pub(crate) struct SparseTrainingModel {
    total_sequences: usize,
    token_frequencies: HashMap<i32, usize>,
}

/// A BM25 model.
#[derive(Debug, Deserialize, Serialize)]
pub struct SparseModel {
    b: f32,
    k1: f32,
    total_sequences: usize,
    average_tokens: f32,
    token_frequencies: HashMap<i32, usize>,
}

impl SparseTrainingModel {
    pub(crate) fn fit(&mut self, token_frequency: HashMap<i32, usize>) {
        self.total_sequences += usize::from(!token_frequency.is_empty());
        for (id, frequency) in token_frequency {
            self.token_frequencies
                .entry(id)
                .and_modify(|frequencies| *frequencies += frequency)
                .or_insert(frequency);
        }
    }

    pub(crate) fn finish(self, b: f32, k1: f32) -> SparseModel {
        #[allow(clippy::cast_precision_loss)]
        let average_tokens = if self.total_sequences == 0 {
            0.
        } else {
            self.token_frequencies.values().sum::<usize>() as f32 / self.total_sequences as f32
        };

        SparseModel {
            b,
            k1,
            total_sequences: self.total_sequences,
            average_tokens,
            token_frequencies: self.token_frequencies,
        }
    }
}

impl SparseModel {
    const DEFAULT_B: f32 = 0.75;
    const DEFAULT_K1: f32 = 1.2;

    /// Creates a sparse model from a configuration.
    pub(crate) fn new<T, P>(config: &Config<T, P>) -> Result<Self, TractError> {
        serde_json::from_reader(BufReader::new(File::open(
            config.dir.join("sparse_model.json"),
        )?))
        .map_err(Into::into)
    }

    /// Fits a new sparse model to a corpus.
    pub fn fit<T, P>(
        config: &Config<T, P>,
        sequences: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<(), TractError>
    where
        T: Tokenize,
    {
        let tokenizer = T::new(config).map_err(|error| anyhow!(error))?;
        let mut model = SparseTrainingModel::default();
        for sequence in sequences {
            let encoding = tokenizer.encode(sequence).map_err(|error| anyhow!(error))?;
            let frequency = encoding.to_token_frequency(tokenizer.special_token_ids())?;
            model.fit(frequency);
        }
        let model = model.finish(Self::DEFAULT_B, Self::DEFAULT_K1);

        serde_json::to_writer(
            BufWriter::new(File::create(config.dir.join("sparse_model.json"))?),
            &model,
        )
        .map_err(Into::into)
    }

    pub(crate) fn run_document(
        &self,
        token_frequency: HashMap<i32, usize>,
    ) -> (Vec<i32>, Vec<f32>) {
        #[allow(clippy::cast_precision_loss)]
        let total_tokens = token_frequency.values().sum::<usize>() as f32;
        let coefficient = self.k1 * (1. - self.b + self.b * total_tokens / self.average_tokens);

        let truncate = token_frequency.len() > 1_000;
        let token_frequency = token_frequency.into_iter().map(move |(id, frequency)| {
            #[allow(clippy::cast_precision_loss)]
            let frequency = frequency as f32;
            (id, frequency / (coefficient + frequency))
        });

        if truncate {
            token_frequency
                .sorted_unstable_by(|(_, f1), (_, f2)| f1.total_cmp(f2).reverse())
                .take(1_000)
                .unzip()
        } else {
            token_frequency.unzip()
        }
    }

    pub(crate) fn run_query(&self, token_ids: HashSet<i32>) -> (Vec<i32>, Vec<f32>) {
        let (ids, mut frequencies) = token_ids
            .into_iter()
            .map(|id| {
                let frequency = self.token_frequencies.get(&id).copied().unwrap_or(1);
                #[allow(clippy::cast_precision_loss)]
                let frequency = ((self.total_sequences + 1) as f32 / (frequency as f32 + 0.5)).ln();
                (id, frequency)
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();
        let total_frequency = frequencies.iter().sum::<f32>();
        for frequency in &mut frequencies {
            *frequency /= total_frequency;
        }

        if ids.len() > 1_000 {
            ids.into_iter()
                .zip(frequencies)
                .sorted_unstable_by(|(_, f1), (_, f2)| f1.abs().total_cmp(&f2.abs()).reverse())
                .take(1_000)
                .unzip()
        } else {
            (ids, frequencies)
        }
    }
}

#[cfg(test)]
mod tests {
    use ndarray::{Array, Array2, Dimension};
    use tract_onnx::prelude::{tvec, DatumType, IntoArcTensor};
    use xayn_test_utils::{assert_approx_eq, asset::smbert_mocked};

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
            .with_token_size(64)
            .unwrap();
        let model = Model::new(&config).unwrap();

        assert_eq!(model.model.model().input_outlets().unwrap().len(), 3);
        let fact = model.model.model().input_fact(0).unwrap();
        assert_eq!(fact.shape.as_concrete().unwrap(), [1, model.token_size]);
        assert_eq!(fact.datum_type, DatumType::I64);
        let fact = model.model.model().input_fact(1).unwrap();
        assert_eq!(fact.shape.as_concrete().unwrap(), [1, model.token_size]);
        assert_eq!(fact.datum_type, DatumType::I64);
        let fact = model.model.model().input_fact(2).unwrap();
        assert_eq!(fact.shape.as_concrete().unwrap(), [1, model.token_size]);
        assert_eq!(fact.datum_type, DatumType::I64);

        assert_eq!(model.model.model().output_outlets().unwrap().len(), 2);
        let fact = model.model.model().output_fact(0).unwrap();
        assert_eq!(
            fact.shape.as_concrete().unwrap(),
            [1, model.token_size, model.embedding_size],
        );
        assert_eq!(fact.datum_type, DatumType::F32);
        let fact = model.model.model().output_fact(1).unwrap();
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

        let inputs = tvec![
            TValue::Const(Array2::from_elem(shape, 0_i64).into_arc_tensor()),
            TValue::Const(Array2::from_elem(shape, 1_i64).into_arc_tensor()),
            TValue::Const(Array2::from_elem(shape, 0_i64).into_arc_tensor()),
        ];
        let prediction = model.predict(inputs).unwrap();
        assert_eq!(model.token_size, shape.1);
        assert_eq!(prediction.shape(), [shape.0, shape.1, model.embedding_size]);
    }

    #[test]
    fn test_sparse_fit() {
        let mut model = SparseTrainingModel::default();
        model.fit(HashMap::default());
        assert_eq!(model.total_sequences, 0);
        assert!(model.token_frequencies.is_empty());

        model.fit([(0, 5), (1, 10)].into());
        assert_eq!(model.total_sequences, 1);
        assert_eq!(model.token_frequencies, [(0, 5), (1, 10)].into());

        model.fit([(1, 2), (3, 4)].into());
        assert_eq!(model.total_sequences, 2);
        assert_eq!(model.token_frequencies, [(0, 5), (1, 12), (3, 4)].into());
    }

    #[test]
    fn test_sparse_finish() {
        let model =
            SparseTrainingModel::default().finish(SparseModel::DEFAULT_B, SparseModel::DEFAULT_K1);
        assert_eq!(model.total_sequences, 0);
        assert_approx_eq!(f32, model.average_tokens, 0.);
        assert!(model.token_frequencies.is_empty());

        let model = SparseTrainingModel {
            total_sequences: 2,
            token_frequencies: [(0, 1), (2, 3)].into(),
        }
        .finish(SparseModel::DEFAULT_B, SparseModel::DEFAULT_K1);
        assert_eq!(model.total_sequences, 2);
        assert_approx_eq!(f32, model.average_tokens, 2.);
        assert_eq!(model.token_frequencies, [(0, 1), (2, 3)].into());
    }

    #[test]
    #[ignore = "sparse model not available"]
    fn test_sparse_new() {
        let config = Config::new(smbert_mocked().unwrap()).unwrap();
        let model = SparseModel::new(&config).unwrap();

        assert!(model.total_sequences != 0);
        assert!(model.average_tokens > 0.);
        assert!(!model.token_frequencies.is_empty());
    }

    #[test]
    fn test_sparse_document() {
        let model = SparseModel {
            b: 1.,
            k1: 1.,
            total_sequences: 3,
            average_tokens: 3.,
            token_frequencies: [(0, 1), (2, 3), (4, 5)].into(),
        };
        let (indices, values) = model.run_document(HashMap::default());
        assert!(indices.is_empty());
        assert!(values.is_empty());

        let (indices, values) = model.run_document([(0, 2), (4, 1)].into());
        let embedding = indices.into_iter().zip(values).collect::<HashMap<_, _>>();
        assert_eq!(embedding.len(), 2);
        assert_approx_eq!(f32, embedding[&0], 0.666_666_7);
        assert_approx_eq!(f32, embedding[&4], 0.5);
    }

    #[test]
    fn test_sparse_query() {
        let model = SparseModel {
            b: 1.,
            k1: 1.,
            total_sequences: 3,
            average_tokens: 3.,
            token_frequencies: [(0, 1), (2, 3), (4, 5)].into(),
        };
        let (indices, values) = model.run_query(HashSet::default());
        assert!(indices.is_empty());
        assert!(values.is_empty());

        let (indices, values) = model.run_query([0, 4].into());
        let embedding = indices.into_iter().zip(values).collect::<HashMap<_, _>>();
        assert_eq!(embedding.len(), 2);
        assert_approx_eq!(f32, embedding[&0], 1.480_775_2);
        assert_approx_eq!(f32, embedding[&4], -0.480_775_2, epsilon = 1e-7);

        let (indices, values) = model.run_query([2, 3].into());
        let embedding = indices.into_iter().zip(values).collect::<HashMap<_, _>>();
        assert_eq!(embedding.len(), 2);
        assert_approx_eq!(f32, embedding[&2], 0.119_827_81, epsilon = 1e-7);
        assert_approx_eq!(f32, embedding[&3], 0.880_172_2);
    }
}
