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

use std::collections::HashMap;

use chrono::{offset::Utc, DateTime, Duration, NaiveDate};
use criterion::{black_box, criterion_group, BatchSize, Criterion};
use itertools::Itertools;
use ndarray::Array1;
use rand::Rng;
use rand_distr::Uniform;

use xayn_discovery_engine_ai::pairwise_cosine_similarity;
use xayn_discovery_engine_core::stack::filters::semantic::{
    condensed_cosine_similarity,
    condensed_date_distance,
    condensed_decay_factor,
    condensed_normalized_distance,
};

fn create_embeddings(n: usize, embedding_size: usize) -> Vec<Array1<f32>> {
    let range = Uniform::new(-10000., 10000.);

    (0..n)
        .map(|_| {
            rand::thread_rng()
                .sample_iter(&range)
                .take(embedding_size)
                .collect::<Array1<f32>>()
        })
        .collect()
}

fn create_dates(n: usize, days_range: usize) -> Vec<DateTime<Utc>> {
    let range = Uniform::new(0, days_range);

    let base_date = NaiveDate::from_ymd(2016, 1, 1).and_hms(0, 0, 0);
    let base_date = DateTime::<Utc>::from_utc(base_date, Utc);

    rand::thread_rng()
        .sample_iter(&range)
        .take(n - 1)
        .map(|days| base_date + Duration::days(days as i64))
        .chain(std::iter::once_with(|| {
            base_date + Duration::days(days_range as i64)
        }))
        .collect()
}

fn bench_cosine_similarities(c: &mut Criterion) {
    let embeddings_count = [100, 500, 2000];
    let embeddings_size = [128, 748];

    let embeddings_count_max: usize = *embeddings_count.iter().max().unwrap();
    let embeddings: HashMap<usize, _> = embeddings_size
        .iter()
        .map(|&size| (size, create_embeddings(embeddings_count_max, size)))
        .collect();
    let embeddings: HashMap<usize, _> = embeddings
        .iter()
        .map(|(&size, embeddings)| {
            (
                size,
                embeddings
                    .iter()
                    .map(|embedding| embedding.view())
                    .collect_vec(),
            )
        })
        .collect();

    embeddings_size
        .iter()
        .cartesian_product(embeddings_count.iter())
        .for_each(|(&size, &n)| {
            let embeddings = embeddings.get(&size).unwrap();
            let embeddings = embeddings.iter().take(n).copied();

            let base_name = format!("cosine_similarity_n{}_s{}", n, size);

            c.bench_function(&format!("pairwise_{}", base_name), |b| {
                b.iter_batched(
                    || black_box(embeddings.clone()),
                    pairwise_cosine_similarity,
                    BatchSize::SmallInput,
                );
            });

            c.bench_function(&format!("condensed_{}", base_name), |b| {
                b.iter_batched(
                    || black_box(embeddings.clone()),
                    condensed_cosine_similarity,
                    BatchSize::SmallInput,
                );
            });
        });
}

fn bench_condensed_date_distance(c: &mut Criterion) {
    let days_count = [100, 500, 2000];
    let days_range = [30, 540];

    let days_count_max: usize = *days_count.iter().max().unwrap();
    let days: HashMap<usize, _> = days_range
        .iter()
        .map(|&range| (range, create_dates(days_count_max, range)))
        .collect();

    days_range
        .iter()
        .cartesian_product(days_count.iter())
        .for_each(|(&range, &n)| {
            let days = days.get(&range).unwrap();
            let days = days.iter().take(n).copied();

            let base_name = &format!("n{}_r{}", n, range);

            let distances = condensed_date_distance(days.clone());

            c.bench_function(&format!("condensed_date_distance_{}", base_name), |b| {
                b.iter_batched(
                    || black_box(days.clone()),
                    condensed_date_distance,
                    BatchSize::SmallInput,
                );
            });

            c.bench_function(&format!("condensed_decay_factor_{}", base_name), |b| {
                b.iter_batched(
                    || black_box(distances.clone()),
                    // the last two argument don't have any impact on performance
                    |distances| condensed_decay_factor(distances, range as f32, 0.1),
                    BatchSize::SmallInput,
                );
            });
        });
}

fn bench_condensed_normalized_distance(c: &mut Criterion) {
    let count = [100, 500, 2000];
    let embedding_size = 128;
    let day_range = 30;

    let count_max: usize = *count.iter().max().unwrap();
    let embeddings = create_embeddings(count_max, embedding_size);
    let days = create_dates(count_max, day_range);

    count.iter().for_each(|&n| {
        let embeddings = embeddings.iter().take(n).map(|embedding| embedding.view());
        let days = days.iter().take(n).copied();

        let distances = condensed_date_distance(days.clone());
        let decay_factor = condensed_decay_factor(distances, day_range as f32, 0.1);
        let similarity = condensed_cosine_similarity(embeddings.clone());

        let base_name = &format!("n{}_s{}_r{}", n, embedding_size, day_range);

        c.bench_function(
            &format!("condensed_normalized_distance_{}", base_name),
            |b| {
                b.iter_batched(
                    || black_box((similarity.clone(), decay_factor.clone())),
                    |(similarity, decay_factor)| {
                        condensed_normalized_distance(similarity, decay_factor)
                    },
                    BatchSize::SmallInput,
                );
            },
        );

        // this mimic normalized_distance, the difference is that in the function we create the iterators
        // for embeddings and dates while here we already have them.
        c.bench_function(
            &format!("normalized_distance_from_iterators_{}", base_name),
            |b| {
                b.iter_batched(
                    || black_box((embeddings.clone(), days.clone())),
                    |(embeddings, days)| {
                        let similarity = condensed_cosine_similarity(embeddings);
                        let distances = condensed_date_distance(days);
                        let decay_factor = condensed_decay_factor(distances, day_range as f32, 0.1);
                        condensed_normalized_distance(similarity, decay_factor)
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    });
}

criterion_group!(b_condensed_date_distance, bench_condensed_date_distance);
criterion_group!(b_cosine_similarity, bench_cosine_similarities);
criterion_group!(
    b_condensed_normalized_distance,
    bench_condensed_normalized_distance
);

fn main() {
    criterion::Criterion::default()
        .configure_from_args()
        .final_summary();

    b_cosine_similarity();
    b_condensed_date_distance();
    b_condensed_normalized_distance();
}
