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

//! Run as `cargo bench --bench mbert --features onnxruntime`.

use std::{io::Result, path::Path};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ndarray::{s, Array1, Axis};
use onnxruntime::{environment::Environment, GraphOptimizationLevel};

use xayn_discovery_engine_bert::{
    kinds::SMBert,
    AveragePooler,
    Config,
    Embedding2,
    FirstPooler,
    NonePooler,
};
use xayn_discovery_engine_test_utils::smbert;
use xayn_discovery_engine_tokenizer::{
    AccentChars,
    Builder as TokenizerBuilder,
    CaseChars,
    ChineseChars,
    ControlChars,
    Padding,
    Truncation,
};

const TOKEN_SIZE: usize = 64;
const SEQUENCE: &str = "This is a sequence.";

macro_rules! bench_tract {
    (
        $manager:expr,
        $group:expr => $kind:ty,
        $vocab:expr,
        $model:expr,
        [$($name:expr => $pooler:ty),+ $(,)?] $(,)?
    ) => {
        let mut group = $manager.benchmark_group(format!("{} {}", $group, TOKEN_SIZE));
        $(
            let pipeline = Config::<$kind, _>::from_files($vocab.unwrap(), $model.unwrap())
                .unwrap()
                .with_accents(AccentChars::Cleanse)
                .with_case(CaseChars::Lower)
                .with_token_size(TOKEN_SIZE)
                .unwrap()
                .with_pooling::<$pooler>()
                .build()
                .unwrap();
            group.bench_function($name, |bencher| {
                bencher.iter(|| pipeline.run(black_box(SEQUENCE)).unwrap())
            });
        )+
    };
}

fn bench_onnx(
    manager: &mut Criterion,
    name: &str,
    vocab: Result<impl AsRef<Path>>,
    model: Result<impl AsRef<Path>>,
) {
    let tokenizer = TokenizerBuilder::from_file(vocab.unwrap())
        .unwrap()
        .with_normalizer(
            ControlChars::Cleanse,
            ChineseChars::Keep,
            AccentChars::Cleanse,
            CaseChars::Lower,
        )
        .with_model("[UNK]", "##", 100)
        .with_post_tokenizer("[CLS]", "[SEP]")
        .with_truncation(Truncation::fixed(TOKEN_SIZE, 0))
        .with_padding(Padding::fixed(TOKEN_SIZE, "[PAD]"))
        .build()
        .unwrap();
    let environment = Environment::builder().build().unwrap();
    let mut session = environment
        .new_session_builder()
        .unwrap()
        .with_optimization_level(GraphOptimizationLevel::DisableAll)
        .unwrap()
        .with_model_from_file(model.unwrap())
        .unwrap();

    manager.bench_function(name, |bencher| {
        bencher.iter(|| {
            let encoding = tokenizer.encode(black_box(SEQUENCE));
            let (token_ids, type_ids, _, _, _, _, attention_mask, _) = encoding.into();
            let inputs = vec![
                Array1::<i64>::from(token_ids).insert_axis(Axis(0)),
                Array1::<i64>::from(attention_mask).insert_axis(Axis(0)),
                Array1::<i64>::from(type_ids).insert_axis(Axis(0)),
            ];
            let outputs = session.run(inputs).unwrap();

            black_box(Embedding2::from(outputs[0].slice(s![0, .., ..]).to_owned()));
        })
    });
}

fn bench_tract_smbert(manager: &mut Criterion) {
    bench_tract!(
        manager,
        "Tract SMBert" => SMBert,
        smbert::vocab(),
        smbert::model(),
        [
            "None Pooler" => NonePooler,
            "First Pooler" => FirstPooler,
            "Average Pooler" => AveragePooler,
        ],
    );
}

fn bench_onnx_smbert(manager: &mut Criterion) {
    bench_onnx(manager, "Onnx SMBert", smbert::vocab(), smbert::model());
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
