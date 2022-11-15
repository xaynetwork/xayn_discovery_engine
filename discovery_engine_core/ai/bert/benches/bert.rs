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

//! Run as `cargo bench --bench bert --features onnxruntime`.

use std::path::Path;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ndarray::s;
use onnxruntime::{environment::Environment, GraphOptimizationLevel};
use xayn_discovery_engine_bert::{tokenizer::Tokenizer, Config, Embedding2, NonePooler};
use xayn_discovery_engine_test_utils::asset::smbert;

const TOKEN_SIZE: usize = 64;
const SEQUENCE: &str = "This is a sequence.";

fn bench_tract_bert(manager: &mut Criterion, name: &str, dir: &Path) {
    let pipeline = Config::new(dir)
        .unwrap()
        .with_token_size(TOKEN_SIZE)
        .unwrap()
        .with_pooler::<NonePooler>()
        .build()
        .unwrap();
    manager.bench_function(name, |bencher| {
        bencher.iter(|| pipeline.run(black_box(SEQUENCE)).unwrap())
    });
}

fn bench_onnx_bert(manager: &mut Criterion, name: &str, dir: &Path) {
    let config = Config::new(dir)
        .unwrap()
        .with_token_size(TOKEN_SIZE)
        .unwrap();
    let tokenizer = Tokenizer::new(&config).unwrap();
    let environment = Environment::builder().build().unwrap();
    let mut session = environment
        .new_session_builder()
        .unwrap()
        .with_optimization_level(GraphOptimizationLevel::DisableAll)
        .unwrap()
        .with_model_from_file(config.dir.join("model.onnx"))
        .unwrap();

    manager.bench_function(name, |bencher| {
        bencher.iter(|| {
            let encoding = tokenizer.encode(black_box(SEQUENCE)).unwrap();
            let inputs = encoding.into();
            let outputs = session.run(inputs).unwrap();

            black_box(Embedding2::from(outputs[0].slice(s![0, .., ..]).to_owned()));
        })
    });
}

fn bench_tract_smbert(manager: &mut Criterion) {
    bench_tract_bert(manager, "Tract SMBert", &smbert().unwrap());
}

fn bench_onnx_smbert(manager: &mut Criterion) {
    bench_onnx_bert(manager, "Onnx SMBert", &smbert().unwrap());
}

criterion_group! {
    name = bench;
    config = Criterion::default();
    targets =
        bench_tract_smbert,
        bench_onnx_smbert,
}

criterion_main! {
    bench,
}
