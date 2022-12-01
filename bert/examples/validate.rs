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

//! Compares Bert models evaluated by the onnx or the tract runtime.
//!
//! Run as `cargo run --release --example validate --features onnxruntime`.

use std::{
    marker::PhantomPinned,
    ops::{Bound, Deref, RangeBounds},
    path::{Path, PathBuf},
    pin::Pin,
};

use csv::Reader;
use indicatif::ProgressBar;
use ndarray::{s, Array1, Array2, ArrayView1, Axis};
use onnxruntime::{environment::Environment, session::Session, GraphOptimizationLevel};
use xayn_ai_bert::{
    tokenizer::Tokenizer,
    Config,
    Embedding2,
    NonePooler,
    Pipeline as BertPipeline,
};
use xayn_ai_test_utils::asset::{smbert, transcripts};

fn main() {
    ValidatorConfig {
        token_size: 90,
        source_rt: Runtime::Onnx,
        source_dir: smbert().unwrap(),
        target_rt: Runtime::Tract,
        target_dir: smbert().unwrap(),
        data: DataConfig {
            talks: transcripts().unwrap(),
            range: ..100,
        },
    }
    .build()
    .validate()
    .print();
}

/// The available runtimes.
enum Runtime {
    Onnx,
    Tract,
}

/// Ted talks data configurations.
struct DataConfig<R: RangeBounds<usize>> {
    /// The path to the talks.
    talks: PathBuf,
    /// The range of talks to use for validation.
    range: R,
}

/// Combined validation configurations.
struct ValidatorConfig<R: RangeBounds<usize>> {
    token_size: usize,
    source_rt: Runtime,
    source_dir: PathBuf,
    target_rt: Runtime,
    target_dir: PathBuf,
    data: DataConfig<R>,
}

impl<R: RangeBounds<usize>> ValidatorConfig<R> {
    /// Builds a validator from this configuration.
    fn build(self) -> Validator {
        Validator::build(self)
    }
}

/// The available Bert model pipelines.
#[allow(clippy::enum_variant_names)]
#[allow(clippy::large_enum_variant)]
enum Pipeline {
    /// A Bert model pipeline for the onnx runtime.
    OnnxBert {
        tokenizer: Tokenizer,
        session: Session<'static>,
        _environment: Pin<Box<(Environment, PhantomPinned)>>,
    },
    /// A Bert model pipeline for the tract runtime.
    TractBert(BertPipeline<NonePooler>),
}

// prevent moving out of the pipeline, since we can't pin the session together with the environment
impl Drop for Pipeline {
    fn drop(&mut self) {}
}

impl Pipeline {
    /// Builds a pipeline from a configuration.
    fn build(rt: Runtime, dir: &Path, token_size: usize) -> Self {
        let config = Config::new(dir)
            .unwrap()
            .with_token_size(token_size)
            .unwrap()
            .with_pooler::<NonePooler>();
        match rt {
            Runtime::Onnx => {
                let tokenizer = Tokenizer::new(&config).unwrap();
                let _environment =
                    Box::pin((Environment::builder().build().unwrap(), PhantomPinned));
                // Safety:
                // - environment is pinned, not unpinnable and dropped after session
                // - session can't be moved out of the pipeline independently from the environment
                let session = unsafe { &*(&_environment.0 as *const Environment) }
                    .new_session_builder()
                    .unwrap()
                    .with_optimization_level(GraphOptimizationLevel::DisableAll)
                    .unwrap()
                    .with_model_from_file(config.extract::<&Path>("model.path").unwrap())
                    .unwrap();

                Self::OnnxBert {
                    tokenizer,
                    session,
                    _environment,
                }
            }
            Runtime::Tract => Self::TractBert(config.build().unwrap()),
        }
    }

    /// Runs the model pipeline to infer the embedding of a sequence.
    fn run(&mut self, sequence: impl AsRef<str>) -> Embedding2 {
        match self {
            Self::OnnxBert {
                tokenizer, session, ..
            } => {
                let encoding = tokenizer.encode(sequence).unwrap();
                let inputs = encoding.into();
                let outputs = session.run(inputs).unwrap();

                outputs[0].slice(s![0, .., ..]).to_owned().into()
            }
            Self::TractBert(pipeline) => pipeline.run(sequence).unwrap(),
        }
    }
}

/// A validator to compare two models based on a set of Ted talks.
struct Validator {
    talks: PathBuf,
    skip: usize,
    take: usize,
    source: Pipeline,
    target: Pipeline,
    errors: Array1<f32>,
}

impl Validator {
    /// Builds a validator from a configuration.
    fn build<R: RangeBounds<usize>>(config: ValidatorConfig<R>) -> Self {
        let talks = config.data.talks;
        let skip = match config.data.range.start_bound() {
            Bound::Included(start) => *start,
            Bound::Excluded(start) => start + 1,
            Bound::Unbounded => 0,
        };
        let take = match config.data.range.end_bound() {
            Bound::Included(end) => end + 1,
            Bound::Excluded(end) => *end,
            Bound::Unbounded => 2467, // total #talks
        } - skip;
        let source = Pipeline::build(config.source_rt, &config.source_dir, config.token_size);
        let target = Pipeline::build(config.target_rt, &config.target_dir, config.token_size);
        let errors = Array1::default(11); // #sentences and mean & std per error

        Self {
            talks,
            skip,
            take,
            source,
            target,
            errors,
        }
    }

    /// Computes the mean of the difference between source and target.
    fn mean_absolute_error(source: &Embedding2, target: &Embedding2) -> f32 {
        (source.deref() - target.deref())
            .mapv(|v| v.abs())
            .mean()
            .unwrap_or_default()
    }

    /// Computes the mean of the difference between source and target relative to the source.
    fn mean_relative_error(source: &Embedding2, target: &Embedding2) -> f32 {
        ((source.deref() - target.deref()) / source.deref())
            .mapv(|v| v.is_finite().then(|| v.abs()).unwrap_or_default())
            .mean()
            .unwrap_or_default()
    }

    /// Computes the mean of the squared difference between source and target.
    fn mean_squared_absolute_error(source: &Embedding2, target: &Embedding2) -> f32 {
        (source.deref() - target.deref())
            .mapv(|v| v.powi(2))
            .mean()
            .unwrap_or_default()
            .sqrt()
    }

    /// Computes the mean of the squared difference between source and target relative to the source.
    fn mean_squared_relative_error(source: &Embedding2, target: &Embedding2) -> f32 {
        ((source.deref() - target.deref()) / source.deref())
            .mapv(|v| v.is_finite().then(|| v.powi(2)).unwrap_or_default())
            .mean()
            .unwrap_or_default()
            .sqrt()
    }

    /// Computes the cosine similarity between source and target.
    fn cosine_similarity(source: &Embedding2, target: &Embedding2) -> f32 {
        let norms =
            source.mapv(|v| v.powi(2)).sum().sqrt() * target.mapv(|v| v.powi(2)).sum().sqrt();
        (norms.is_finite() && norms > 0.0)
            .then(|| (source.deref() * target.deref() / norms).sum())
            .unwrap_or_default()
    }

    /// Computes various errors between source and target embeddings based on the chosen ted talks.
    fn validate(&mut self) -> &mut Self {
        let mut reader = Reader::from_path(self.talks.as_path()).unwrap();
        let progress = ProgressBar::new(self.take as u64);
        let mut errors = Array2::<f32>::default((330644, 5)); // total #sentences
        let mut idx = 0;

        for record in reader.records().skip(self.skip).take(self.take) {
            for sequence in record.unwrap()[0].split_inclusive(&['.', '!', '?'] as &[char]) {
                let source = self.source.run(sequence);
                let target = self.target.run(sequence);
                errors.slice_mut(s![idx, ..]).assign(&ArrayView1::from(&[
                    Self::mean_absolute_error(&source, &target),
                    Self::mean_relative_error(&source, &target),
                    Self::mean_squared_absolute_error(&source, &target),
                    Self::mean_squared_relative_error(&source, &target),
                    Self::cosine_similarity(&source, &target),
                ]));
                idx += 1;
            }
            progress.inc(1);
        }
        progress.finish();

        self.errors[0] = idx as f32;
        self.errors
            .slice_mut(s![1..6])
            .assign(&errors.slice(s![..idx, ..]).mean_axis(Axis(0)).unwrap());
        self.errors
            .slice_mut(s![6..])
            .assign(&errors.slice(s![..idx, ..]).std_axis(Axis(0), 1.0));

        self
    }

    /// Prints the validation results to stdout.
    fn print(&self) {
        println!(
            "Validated models on {} talks with {} sentences.",
            self.take, self.errors[0],
        );
        println!(
            "  Mean absolute error: μ = {}, σ = {}",
            self.errors[1], self.errors[6],
        );
        println!(
            "  Mean relative error: μ = {}, σ = {}",
            self.errors[2], self.errors[7],
        );
        println!(
            "  Mean squared absolute error: μ = {}, σ = {}",
            self.errors[3], self.errors[8],
        );
        println!(
            "  Mean squared relative error: μ = {}, σ = {}",
            self.errors[4], self.errors[9],
        );
        println!(
            "  Cosine similarity: μ = {}, σ = {}",
            self.errors[5], self.errors[10],
        );
    }
}
