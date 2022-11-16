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
use xayn_ai_coi::pairwise_cosine_similarity;
use xayn_discovery_engine::{
    document::{Document, NewsResource},
    stack::filters::semantic::{
        condensed_date_distance,
        condensed_decay_factor,
        normalized_distance,
        SemanticFilterConfig,
    },
};

fn create_embeddings(n: usize, embedding_size: usize) -> impl Iterator<Item = Array1<f32>> {
    let range = Uniform::new(-10000., 10000.);

    (0..n).map(move |_| {
        rand::thread_rng()
            .sample_iter(&range)
            .take(embedding_size)
            .collect::<Array1<f32>>()
    })
}

fn create_dates(n: usize, days_range: usize) -> impl Iterator<Item = DateTime<Utc>> {
    let range = Uniform::new(0, days_range);

    let base_date = NaiveDate::from_ymd_opt(2016, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let base_date = DateTime::<Utc>::from_utc(base_date, Utc);

    rand::thread_rng()
        .sample_iter(range)
        .take(n - 1)
        .map(move |days| base_date + Duration::days(days as i64))
        .chain(std::iter::once_with(move || {
            base_date + Duration::days(days_range as i64)
        }))
}

fn create_documents(
    n: usize,
    embedding_size: usize,
    days_range: usize,
) -> impl Iterator<Item = Document> {
    let embeddings = create_embeddings(n, embedding_size);
    let dates = create_dates(n, days_range);

    embeddings.zip(dates).map(|(embedding, date)| {
        use uuid::Uuid;

        Document {
            id: Uuid::new_v4().into(),
            stack_id: Uuid::new_v4().into(),
            bert_embedding: embedding.into(),
            reaction: None,
            resource: NewsResource {
                country: "gb".to_string(),
                date_published: date,
                image: None,
                language: "en".to_string(),
                rank: 0,
                score: None,
                snippet: "".to_string(),
                source_domain: "".to_string(),
                title: "".to_string(),
                topic: "".to_string(),
                url: url::Url::parse("http://localhost").unwrap(),
            },
        }
    })
}

fn bench_cosine_similarities(c: &mut Criterion) {
    let embeddings_count = [100, 500, 2000];
    let embeddings_size = [128, 748];

    let embeddings_count_max: usize = *embeddings_count.iter().max().unwrap();
    let embeddings: HashMap<usize, _> = embeddings_size
        .iter()
        .map(|&size| {
            (
                size,
                create_embeddings(embeddings_count_max, size).collect_vec(),
            )
        })
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

    embeddings_count
        .iter()
        .cartesian_product(embeddings_size.iter())
        .for_each(|(&n, &size)| {
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
        });
}

fn bench_condensed_date_distance(c: &mut Criterion) {
    let days_count = [100, 500, 2000];
    let days_range = [30, 540];

    let days_count_max: usize = *days_count.iter().max().unwrap();
    let days: HashMap<usize, _> = days_range
        .iter()
        .map(|&range| (range, create_dates(days_count_max, range).collect_vec()))
        .collect();

    days_count
        .iter()
        .cartesian_product(days_range.iter())
        .for_each(|(&n, &range)| {
            let days = days.get(&range).unwrap();
            let days = days.iter().take(n).copied();

            let base_name = &format!("n{}_r{}", n, range);

            let distances = condensed_date_distance(days.clone());

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

fn bench_normalized_distance(c: &mut Criterion) {
    let count = [100, 500, 2000];
    let embeddings_size = [128, 748];
    let day_range = 30;

    let count_max: usize = *count.iter().max().unwrap();
    let documents_by_embedding_size: HashMap<usize, _> = embeddings_size
        .iter()
        .map(|&size| {
            (
                size,
                create_documents(count_max, size, day_range).collect_vec(),
            )
        })
        .collect();

    count
        .iter()
        .cartesian_product(embeddings_size.iter())
        .for_each(|(&n, &embedding_size)| {
            let documents: &Vec<Document> =
                documents_by_embedding_size.get(&embedding_size).unwrap();
            let documents = &documents[..n];

            let base_name = &format!("n{}_s{}_r{}", n, embedding_size, day_range);

            c.bench_function(&format!("normalized_distance_{}", base_name), |b| {
                b.iter_batched(
                    || black_box((documents, SemanticFilterConfig::default())),
                    |(documents, config)| normalized_distance(documents, &config),
                    BatchSize::SmallInput,
                );
            });
        });
}

criterion_group!(b_condensed_date_distance, bench_condensed_date_distance);
criterion_group!(b_cosine_similarity, bench_cosine_similarities);
criterion_group!(b_normalized_distance, bench_normalized_distance);

fn main() {
    criterion::Criterion::default()
        .configure_from_args()
        .final_summary();

    b_cosine_similarity();
    b_condensed_date_distance();
    b_normalized_distance();
}
