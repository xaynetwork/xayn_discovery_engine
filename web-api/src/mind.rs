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
use std::{collections::HashMap, fs::File, path::Path};

use csv::{DeserializeRecordsIntoIter, Reader, ReaderBuilder};
use itertools::Itertools;
use ndarray::{Array, ArrayView};
use rand::{seq::IteratorRandom, seq::SliceRandom, thread_rng};
use serde::Deserialize;
use xayn_ai_coi::nan_safe_f32_cmp_desc;

#[derive(Debug, Deserialize)]
struct Impression {
    id: String,
    user_id: String,
    time: String,
    clicks: String,
    news: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Article {
    id: String,
    category: String,
    subcategory: String,
    title: String,
    snippet: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct ArticleProvider {
    articles: HashMap<String, Article>,
}

impl ArticleProvider {
    fn new(path: &str) -> Result<Self, anyhow::Error> {
        let mut articles = HashMap::new();
        for article in read::<Article>(path)? {
            let article = article?;
            articles.insert(article.id.clone(), article);
        }
        Ok(Self { articles })
    }

    fn sample(&self, n: usize) -> Vec<Article> {
        let mut rng = thread_rng();
        self.articles
            .values()
            .cloned()
            .collect_vec()
            .iter()
            .choose_multiple(&mut rng, n)
            .into_iter()
            .cloned()
            .collect()
    }

    fn get(&self, id: &str) -> Option<Article> {
        self.articles.get(id).cloned()
    }
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
    let article_provider = ArticleProvider::new("news.tsv")?;

    let impressions = read("behaviors.tsv")?;

    let nranks = vec![3];
    let mut ndcgs = Array::zeros((nranks.len(), 0));

    // Loop over all impressions, prepare reranker with news in click history
    // and rerank the news in an impression
    for impression in impressions {
        let impression: Impression = impression?;

        // Placeholder for interacting with the entire click history
        for click in impression.clicks.split(' ') {
            match article_provider.get(click) {
                Some(article) => println!("The article {:?} was interacted.", article),
                None => println!("Article id {} not found.", click),
            }
        }

        // Placeholder for reranking the results
        let mut snippet_label_pairs = impression
            .news
            .split(' ')
            .filter_map(|x| {
                x.split_once('-').and_then(|(id, label)| {
                    article_provider.get(id).map(|article| {
                        SnippetLabelPair(article.snippet.to_string(), label.to_string())
                    })
                })
            })
            .collect::<Vec<_>>();
        snippet_label_pairs.shuffle(&mut thread_rng());

        let labels = snippet_label_pairs
            .iter()
            .map(|snippet_label| snippet_label.1.parse::<f32>().unwrap())
            .collect::<Vec<_>>();
        let ndcgs_iteration = ndcg(&labels, &nranks);

        ndcgs
            .push_column(ArrayView::from(&ndcgs_iteration))
            .unwrap();
    }
    println!("{:?}", ndcgs);

    Ok(())
}

fn ndcg(relevance: &[f32], k: &[usize]) -> Vec<f32> {
    let mut optimal_order = relevance.to_owned();
    optimal_order.sort_by(nan_safe_f32_cmp_desc);
    let last = k
        .iter()
        .max()
        .copied()
        .map_or_else(|| relevance.len(), |k| k.min(relevance.len()));
    relevance
        .iter()
        .zip(optimal_order)
        .take(last)
        .scan(
            (1_f32, 0., 0.),
            |(i, dcg, ideal_dcg), (relevance, optimal_order)| {
                *i += 1.;
                let log_i = (*i).log2();
                *dcg += (2_f32.powf(*relevance) - 1.) / log_i;
                *ideal_dcg += (2_f32.powf(optimal_order) - 1.) / log_i;
                Some(*dcg / (*ideal_dcg + 0.00001))
            },
        )
        .enumerate()
        .filter_map(|(i, ndcg)| k.contains(&(i + 1)).then_some(ndcg))
        .collect()
}

fn main() {
    if let Err(e) = run_benchmark() {
        eprintln!("{}", e);
    }
}
