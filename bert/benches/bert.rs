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

use std::{hint::black_box, path::Path};

use criterion::{criterion_group, criterion_main, Criterion};
use xayn_ai_bert::{Config, NonePooler};
use xayn_test_utils::asset::smbert;

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
        bencher.iter(|| black_box(pipeline.run(black_box(SEQUENCE)).unwrap()))
    });
}

fn bench_tract_smbert(manager: &mut Criterion) {
    bench_tract_bert(manager, "Tract SMBert", &smbert().unwrap());
}

criterion_group! {
    name = bench;
    config = Criterion::default();
    targets =
        bench_tract_smbert,
}

criterion_main! {
    bench,
}
