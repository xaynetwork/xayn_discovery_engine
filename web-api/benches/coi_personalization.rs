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

use std::{collections::HashMap, hint::black_box, time::Duration};

use chrono::Utc;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use itertools::Itertools;
use rand::{
    distributions::{Distribution, Uniform},
    rngs::SmallRng,
    SeedableRng,
};
use xayn_ai_bert::Embedding1;
use xayn_ai_coi::{Coi, CoiConfig, CoiId, CoiStats};
use xayn_web_api::bench_rerank;

fn tag(i: usize) -> String {
    format!("tag {i}")
}

macro_rules! bench_rerank {
    ($(
        $function: ident,
        $embedding_size: expr,
        $document_size: expr,
        $interest_size: expr
    );+ $(;)?) => {$(
        fn $function(c: &mut Criterion) {
            let mut rng = SmallRng::from_entropy();
            let floats = Uniform::new_inclusive(-1.0, 1.0);
            let ints = Uniform::new(0, $interest_size);

            let system = CoiConfig::default().build();
            let timestamp = Utc::now();

            let documents = (0..$document_size)
                .map(|_| {
                    let embedding = Embedding1::from(
                        floats
                            .sample_iter(&mut rng)
                            .take($embedding_size)
                            .collect_vec(),
                    )
                    .normalize()
                    .unwrap();
                    let tags = vec![tag(ints.sample(&mut rng))];
                    (embedding, tags)
                })
                .collect_vec();

            let interests = (0..$interest_size)
                .map(|i| {
                    let id = CoiId::new();
                    let point = Embedding1::from(
                        floats
                            .sample_iter(&mut rng)
                            .take($embedding_size)
                            .collect_vec(),
                    )
                    .normalize()
                    .unwrap();
                    let stats = CoiStats {
                        view_count: i,
                        view_time: Duration::from_secs(i as u64),
                        last_view: timestamp,
                    };
                    Coi { id, point, stats }
                })
                .collect_vec();

            let tag_weights = (0..$interest_size).map(|i| (tag(i), i)).collect::<HashMap<_, _>>();

            let name = format!(
                "rerank {} documents on {} interests (embedding size: {})",
                $document_size,
                $interest_size,
                $embedding_size,
            );
            c.bench_function(
                &name,
                |b| b.iter_batched(
                    || (documents.clone(), tag_weights.clone()),
                    |(documents, tag_weights)| bench_rerank(
                        black_box(&system),
                        black_box(documents),
                        black_box(&interests),
                        black_box(tag_weights),
                        black_box(timestamp),
                    ),
                    BatchSize::SmallInput,
                ),
            );
        }
    )+};
}

bench_rerank! {
    bench_rerank_128_0_2, 128, 0, 2;
    bench_rerank_128_10_2, 128, 10, 2;
    bench_rerank_128_20_2, 128, 20, 2;
    bench_rerank_128_30_2, 128, 30, 2;
    bench_rerank_128_40_2, 128, 40, 2;

    bench_rerank_128_0_5, 128, 0, 5;
    bench_rerank_128_10_5, 128, 10, 5;
    bench_rerank_128_20_5, 128, 20, 5;
    bench_rerank_128_30_5, 128, 30, 5;
    bench_rerank_128_40_5, 128, 40, 5;

    bench_rerank_128_0_10, 128, 0, 10;
    bench_rerank_128_10_10, 128, 10, 10;
    bench_rerank_128_20_10, 128, 20, 10;
    bench_rerank_128_30_10, 128, 30, 10;
    bench_rerank_128_40_10, 128, 40, 10;
}

criterion_group!(
    rerank_with_2_interests,
    bench_rerank_128_0_2,
    bench_rerank_128_10_2,
    bench_rerank_128_20_2,
    bench_rerank_128_30_2,
    bench_rerank_128_40_2,
);

criterion_group!(
    rerank_with_5_interests,
    bench_rerank_128_0_5,
    bench_rerank_128_10_5,
    bench_rerank_128_20_5,
    bench_rerank_128_30_5,
    bench_rerank_128_40_5,
);

criterion_group!(
    rerank_with_10_interests,
    bench_rerank_128_0_10,
    bench_rerank_128_10_10,
    bench_rerank_128_20_10,
    bench_rerank_128_30_10,
    bench_rerank_128_40_10,
);

criterion_main!(
    rerank_with_2_interests,
    rerank_with_5_interests,
    rerank_with_10_interests,
);
