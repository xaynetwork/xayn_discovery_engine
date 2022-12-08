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
use rand::{
    seq::{IteratorRandom, SliceRandom},
    thread_rng,
};
use serde::{de, Deserialize, Deserializer};
use xayn_ai_coi::nan_safe_f32_cmp_desc;

use crate::models::DocumentId;

#[derive(Debug, Deserialize)]
struct ViewedDocument {
    document_id: DocumentId,
    was_clicked: bool,
}

impl ViewedDocument {
    fn new(document_id: DocumentId, was_clicked: bool) -> Self {
        Self {
            document_id,
            was_clicked,
        }
    }
}

fn deserialize_viewed_documents<'de, D>(deserializer: D) -> Result<Vec<ViewedDocument>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let viewed_documents = s.split(' ');
    viewed_documents
        .map(|viewed_document| -> Result<_, D::Error> {
            let mut parts = viewed_document.split('-');
            let document_id = parts
                .next()
                .ok_or_else(|| de::Error::custom("missing document id"))?;
            let document_id = DocumentId::new(document_id).map_err(de::Error::custom)?;
            let was_clicked = parts
                .next()
                .ok_or_else(|| de::Error::custom("missing was_clicked"))?;
            let was_clicked = match was_clicked {
                "0" => Ok(false),
                "1" => Ok(true),
                _ => Err(de::Error::custom("invalid was_clicked")),
            }?;
            Ok(ViewedDocument::new(document_id, was_clicked))
        })
        .collect()
}

fn deserialize_clicked_documents<'de, D>(deserializer: D) -> Result<Vec<DocumentId>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let viewed_documents = s.split(' ');
    viewed_documents
        .map(|document| -> Result<_, D::Error> {
            let document_id = DocumentId::new(document).map_err(de::Error::custom)?;
            Ok(document_id)
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct Impression {
    id: String,
    user_id: String,
    time: String,
    #[serde(deserialize_with = "deserialize_clicked_documents")]
    clicks: Vec<DocumentId>,
    #[serde(deserialize_with = "deserialize_viewed_documents")]
    news: Vec<ViewedDocument>,
}

#[derive(Clone, Debug, Deserialize)]
struct Document {
    id: DocumentId,
    category: String,
    subcategory: String,
    title: String,
    snippet: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct DocumentProvider {
    articles: HashMap<DocumentId, Document>,
}

impl DocumentProvider {
    fn new(path: &str) -> Result<Self, anyhow::Error> {
        let articles = read::<Document>(path)?
            .map(|article| article.map(|article| (article.id.clone(), article)))
            .try_collect()?;
        Ok(Self { articles })
    }

    fn sample(&self, n: usize) -> Vec<&Document> {
        self.articles.values().choose_multiple(&mut thread_rng(), n)
    }

    fn get(&self, id: &DocumentId) -> Option<&Document> {
        self.articles.get(id)
    }
}

struct SnippetLabelPair(String, bool);

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
    let article_provider = DocumentProvider::new("news.tsv")?;

    let impressions = read("behaviors.tsv")?;

    let nranks = vec![3];
    let mut ndcgs = Array::zeros((nranks.len(), 0));

    // Loop over all impressions, prepare reranker with news in click history
    // and rerank the news in an impression
    for impression in impressions {
        let impression: Impression = impression?;

        // Placeholder for interacting with the entire click history
        for click in impression.clicks {
            match article_provider.get(&click) {
                Some(article) => println!("The article {:?} was interacted.", article),
                None => println!("Article id {} not found.", click),
            }
        }

        // Placeholder for reranking the results
        let mut snippet_label_pairs = impression
            .news
            .iter()
            .filter_map(|viewed_document| {
                article_provider
                    .get(&viewed_document.document_id)
                    .map(|article| {
                        SnippetLabelPair(article.snippet.clone(), viewed_document.was_clicked)
                    })
            })
            .collect::<Vec<_>>();
        snippet_label_pairs.shuffle(&mut thread_rng());

        let labels = snippet_label_pairs
            .iter()
            .map(|snippet_label| if snippet_label.1 { 1.0 } else { 0.0 })
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
