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

use std::{hint::black_box, time::Duration};

use chrono::Utc;
use criterion::{criterion_group, BatchSize, Criterion};
use itertools::Itertools;
use rand::Rng;
use rand_distr::Uniform;
use xayn_ai_coi::{compute_coi_decay_factor, compute_coi_relevances, Coi, CoiId};

fn create_cois(n: usize, embedding_size: usize) -> Vec<Coi> {
    let range = Uniform::new(-1., 1.);
    let now = Utc::now();

    (0..n)
        .map(|_| {
            let point = rand::thread_rng()
                .sample_iter(&range)
                .take(embedding_size)
                .collect_vec()
                .try_into()
                .unwrap();
            Coi::new(CoiId::new(), point, now)
        })
        .collect()
}

fn bench_compute_coi_decay_factor(c: &mut Criterion) {
    let horizon = Duration::new(60 * 60 * 24 * 30, 0); // 30 days
    let now = Utc::now();
    let last_view = now - chrono::Duration::seconds(60 * 60);

    c.bench_function("compute_coi_decay_factor", |b| {
        b.iter(|| {
            black_box(compute_coi_decay_factor(
                black_box(horizon),
                black_box(now),
                black_box(last_view),
            ))
        })
    });
}

fn bench_compute_coi_relevance(c: &mut Criterion) {
    let count = [100, 500, 2000];
    let embedding_size = 128;
    let horizon = Duration::new(60 * 60 * 24 * 30, 0); // 30 days
    let now = Utc::now();

    let count_max: usize = *count.iter().max().unwrap();
    let cois = create_cois(count_max, embedding_size);

    count.iter().for_each(|&n| {
        let cois = &cois[..n];

        let base_name = &format!("n{n}_s{embedding_size}");

        c.bench_function(&format!("compute_coi_relevance_{base_name}"), |b| {
            b.iter_batched(
                || black_box(cois),
                |cois| {
                    black_box(compute_coi_relevances(
                        black_box(cois),
                        black_box(horizon),
                        black_box(now),
                    ))
                },
                BatchSize::SmallInput,
            );
        });
    });
}

criterion_group!(b_compute_coi_decay_factor, bench_compute_coi_decay_factor);
criterion_group!(b_compute_coi_relevance, bench_compute_coi_relevance);

fn main() {
    criterion::Criterion::default()
        .configure_from_args()
        .final_summary();

    b_compute_coi_decay_factor();
    b_compute_coi_relevance();
}
