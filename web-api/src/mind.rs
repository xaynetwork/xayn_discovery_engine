// Copyright 2022 Xayn AG
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
#![allow(dead_code)]

//! Executes the user-based MIND benchmark.
use std::{cmp::min, collections::HashMap, fs::File, path::Path};

use csv::{DeserializeRecordsIntoIter, Reader, ReaderBuilder};
use rand::{seq::SliceRandom, thread_rng};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Impression {
    id: String,
    user_id: String,
    time: String,
    clicks: String,
    news: String,
}

#[derive(Debug, Deserialize)]
struct Article {
    id: String,
    category: String,
    subcategory: String,
    title: String,
    snippet: String,
    url: String,
}

struct SnippetLabelPair(String, String);

fn read<T>(path: &str) -> Result<DeserializeRecordsIntoIter<File, T>, anyhow::Error>
where
    for<'de> T: Deserialize<'de>,
{
    Ok(read_from_tsv(path)?.into_deserialize())
}

fn read_from_tsv<P>(path: P) -> Result<Reader<File>, anyhow::Error>
where
    P: AsRef<Path>,
{
    ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .from_path(path)
        .map_err(Into::into)
}

/// Runs the user-based mind benchmark
fn run_benchmark() -> Result<(), anyhow::Error> {
    let articles = read("news.tsv")?
        .map(|result| result.map(|article: Article| (article.id.clone(), article)))
        .collect::<Result<HashMap<_, _>, _>>()?;

    let impressions_iter = read("behaviors.tsv")?;

    let nranks = vec![3];
    let mut ndcgs: Vec<Vec<f32>> = vec![vec![]; nranks.len()];

    // Loop over all impressions, prepare reranker with news in click history
    // and rerank the news in an impression
    for impression in impressions_iter {
        let impression: Impression = impression?;
        let clicks_iter = impression.clicks.split(' ');

        // Placeholder for interacting with the entire click history
        clicks_iter.for_each(|click| match articles.get(click) {
            Some(article) => println!("The article {:?} was interacted.", article),
            None => println!("Article id {} not found.", click),
        });

        let news = impression.news.split(' ').collect::<Vec<&str>>();

        // Placeholder for reranking the results
        let news_labels_iter = news.iter().map(|x| x.split('-').collect::<Vec<_>>());

        let mut snippet_label_pairs = news_labels_iter
            .map(|id_label| match articles.get(id_label[0]) {
                Some(article) => {
                    SnippetLabelPair((*article.snippet).to_string(), id_label[1].to_string())
                }
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        snippet_label_pairs.shuffle(&mut thread_rng());

        let labels = snippet_label_pairs
            .iter()
            .map(|snippet_label| snippet_label.1.parse::<f32>().unwrap())
            .collect::<Vec<_>>();
        let ndcgs_iteration = ndcg(&labels[..], &nranks);

        ndcgs_iteration
            .iter()
            .enumerate()
            .for_each(|(i, ndcg)| ndcgs[i].push(*ndcg));

        println!("{:?}", ndcgs_iteration);
    }

    Ok(())
}

fn ndcg(relevance: &[f32], k: &[usize]) -> Vec<f32> {
    let mut optimal_order = relevance.to_owned();
    optimal_order.sort_by(|a, b| b.partial_cmp(a).unwrap());

    let last = min(*k.iter().max().unwrap() as usize, relevance.len());

    let mut out = Vec::new();
    let mut dcg: f32 = 0.0;
    let mut ideal_dcg: f32 = 0.0;

    #[allow(clippy::cast_precision_loss)] // small numbers
    for i in 0..last {
        dcg += (2f32.powf(relevance[i]) - 1.0) / (i as f32 + 2.0).log2();
        ideal_dcg += (2f32.powf(optimal_order[i]) - 1.0) / ((i as f32) + 2.0).log2();

        out.push(dcg / (ideal_dcg + 0.00001));
    }
    out.into_iter()
        .enumerate()
        .filter(|&(i, _)| k.contains(&(i + 1)))
        .map(|(_, e)| e)
        .collect::<Vec<f32>>()
}

fn main() {
    if let Err(e) = run_benchmark() {
        eprintln!("{}", e);
    }
}
