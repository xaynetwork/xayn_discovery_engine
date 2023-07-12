// Copyright 2023 Xayn AG
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

use std::hint::black_box;

use chrono::Utc;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use itertools::Itertools;
use xayn_ai_bert::Embedding1;
use xayn_ai_coi::CoiConfig;
use xayn_web_api::bench_derive_interests;

macro_rules! bench_identical {
    ($($function: ident, $embedding_size: expr, $interest_size: expr);+ $(;)?) => {$(
        fn $function(c: &mut Criterion) {
            let system = CoiConfig::default().build();
            let timestamp = Utc::now();
            let embedding = Embedding1::from(vec![1.0; $embedding_size])
                .normalize()
                .unwrap();
            let history = vec![(timestamp, embedding); $interest_size];

            let name = format!(
                "derive {} interests with identical embeddings ({})",
                $interest_size,
                $embedding_size,
            );
            c.bench_function(
                &name,
                |b| b.iter_batched(
                    || history.clone(),
                    |history| bench_derive_interests(black_box(&system), black_box(history)),
                    BatchSize::SmallInput,
                ),
            );
        }
    )+};
}

bench_identical! {
    bench_derive_interests_with_identical_embedding_128_0, 128, 0;
    bench_derive_interests_with_identical_embedding_128_10, 128, 10;
    bench_derive_interests_with_identical_embedding_128_20, 128, 20;
    bench_derive_interests_with_identical_embedding_128_30, 128, 30;
    bench_derive_interests_with_identical_embedding_128_40, 128, 40;
}

criterion_group!(
    interests_with_identiccal_embeddings,
    bench_derive_interests_with_identical_embedding_128_0,
    bench_derive_interests_with_identical_embedding_128_10,
    bench_derive_interests_with_identical_embedding_128_20,
    bench_derive_interests_with_identical_embedding_128_30,
    bench_derive_interests_with_identical_embedding_128_40,
);

macro_rules! bench_orthogonal {
    ($($function: ident, $embedding_size: expr, $interest_size: expr);+ $(;)?) => {$(
        fn $function(c: &mut Criterion) {
            let system = CoiConfig::default().build();
            let timestamp = Utc::now();
            let history = (0..$interest_size)
                .map(|i| {
                    let mut embedding = vec![0.0; $embedding_size];
                    embedding[i] = 1.0;
                    let embedding = Embedding1::from(embedding).normalize().unwrap();
                    (timestamp, embedding)
                })
                .collect_vec();

            let name = format!(
                "derive {} interests with orthogonal embeddings ({})",
                $interest_size,
                $embedding_size,
            );
            c.bench_function(
                &name,
                |b| b.iter_batched(
                    || history.clone(),
                    |history| bench_derive_interests(black_box(&system), black_box(history)),
                    BatchSize::SmallInput,
                ),
            );
        }
    )+};
}

bench_orthogonal! {
    bench_derive_interests_with_orthogonal_embedding_128_0, 128, 0;
    bench_derive_interests_with_orthogonal_embedding_128_10, 128, 10;
    bench_derive_interests_with_orthogonal_embedding_128_20, 128, 20;
    bench_derive_interests_with_orthogonal_embedding_128_30, 128, 30;
    bench_derive_interests_with_orthogonal_embedding_128_40, 128, 40;
}

criterion_group!(
    interests_with_orthogonal_embeddings,
    bench_derive_interests_with_orthogonal_embedding_128_0,
    bench_derive_interests_with_orthogonal_embedding_128_10,
    bench_derive_interests_with_orthogonal_embedding_128_20,
    bench_derive_interests_with_orthogonal_embedding_128_30,
    bench_derive_interests_with_orthogonal_embedding_128_40,
);

criterion_main!(
    interests_with_identiccal_embeddings,
    interests_with_orthogonal_embeddings,
);
