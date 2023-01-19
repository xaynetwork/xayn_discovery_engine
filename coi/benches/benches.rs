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

use std::time::{Duration, SystemTime};

use criterion::{black_box, criterion_group, BatchSize, Criterion};
use itertools::Itertools;
use rand::Rng;
use rand_distr::Uniform;
use xayn_ai_coi::{
    compute_coi_relevances,
    stats::compute_coi_decay_factor,
    CoiId,
    CoiPoint,
    PositiveCoi,
};

fn create_positive_coi(n: usize, embedding_size: usize) -> Vec<PositiveCoi> {
    let range = Uniform::new(-1., 1.);

    (0..n)
        .map(|_| {
            let point = rand::thread_rng()
                .sample_iter(&range)
                .take(embedding_size)
                .collect_vec()
                .try_into()
                .unwrap();
            PositiveCoi::new(CoiId::new(), point)
        })
        .collect()
}

fn bench_compute_coi_decay_factor(c: &mut Criterion) {
    let horizon = black_box(Duration::new(3600 * 24 * 30, 0)); // 30 days
    let now = black_box(SystemTime::now());
    let last_view = black_box(
        SystemTime::now()
            .checked_sub(Duration::new(3600, 0))
            .unwrap(),
    );

    c.bench_function("compute_coi_decay_factor", |b| {
        b.iter(|| compute_coi_decay_factor(horizon, now, last_view))
    });
}

fn bench_compute_coi_relevance(c: &mut Criterion) {
    let count = [100, 500, 2000];
    let embedding_size = 128;
    let horizon = black_box(Duration::new(3600 * 24 * 30, 0)); // 30 days
    let now = black_box(SystemTime::now());

    let count_max: usize = *count.iter().max().unwrap();
    let positive_cois = create_positive_coi(count_max, embedding_size);

    count.iter().for_each(|&n| {
        let positive_cois = &positive_cois[..n];

        let base_name = &format!("n{}_s{}", n, embedding_size);

        c.bench_function(&format!("compute_coi_relevance_{}", base_name), |b| {
            b.iter_batched(
                || black_box(positive_cois),
                |cois| compute_coi_relevances(cois, horizon, now),
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
