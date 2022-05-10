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

//! Run as `cargo bench --bench kpe`.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use kpe::{Config, Pipeline};
use xayn_discovery_engine_test_utils::kpe::{bert, classifier, cnn, vocab};

fn bench_kpe(manager: &mut Criterion) {
    let config = Config::from_files(
        vocab().unwrap(),
        bert().unwrap(),
        cnn().unwrap(),
        classifier().unwrap(),
    )
    .unwrap()
    .with_token_size(128)
    .unwrap();
    let pipeline = Pipeline::from(config).unwrap();

    let sequence = "This sequence will be split into key phrases.";
    manager.bench_function("KPE", |bencher| {
        bencher.iter(|| pipeline.run(black_box(sequence)).unwrap())
    });
}

criterion_group! {
    name = bench;
    config = Criterion::default();
    targets =
        bench_kpe,
}

criterion_main! {
    bench,
}
