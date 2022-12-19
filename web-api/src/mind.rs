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

//! Executes the user-based MIND benchmark.

#![allow(dead_code)]

use std::{collections::HashMap, fs::File, path::Path};

use anyhow::Error;
use csv::{DeserializeRecordsIntoIter, Reader, ReaderBuilder};
use itertools::Itertools;
use ndarray::{Array, ArrayView};
use rand::{
    seq::{IteratorRandom, SliceRandom},
    thread_rng,
};
use serde::{de, Deserialize, Deserializer};
use xayn_ai_coi::{nan_safe_f32_cmp_desc, CoiConfig, CoiSystem};

use crate::{
    embedding::{self, Embedder},
    models::{DocumentId, DocumentProperties, IngestedDocument, UserId, UserInteractionType},
    personalization::{
        routes::{
            personalize_documents_by,
            update_interactions,
            PersonalizeBy,
            UserInteractionData,
        },
        PersonalizationConfig,
    },
    storage::{self, memory::Storage},
};

struct State {
    storage: Storage,
    embedder: Embedder,
    coi: CoiSystem,
    personalization: PersonalizationConfig,
}

impl State {
    fn new(storage: Storage) -> Result<Self, Error> {
        let embedder = Embedder::load(&embedding::Config {
            directory: "../assets/smbert_v0003".into(),
            ..embedding::Config::default()
        })?;
        let coi = CoiConfig::default().build();
        let personalization = PersonalizationConfig::default();

        Ok(Self {
            storage,
            embedder,
            coi,
            personalization,
        })
    }

    async fn insert(&self, documents: Vec<Document>) -> Result<(), Error> {
        let documents = documents
            .into_iter()
            .map(|document| {
                let document = IngestedDocument {
                    id: document.id,
                    snippet: document.snippet,
                    properties: DocumentProperties::default(),
                    tags: vec![document.category, document.subcategory],
                };
                let embedding = self.embedder.run(&document.snippet)?;

                Ok((document, embedding))
            })
            .try_collect::<_, _, Error>()?;

        storage::Document::insert(&self.storage, documents)
            .await
            .map_err(Into::into)
    }

    async fn interact(&self, user: &UserId, documents: &[DocumentId]) -> Result<(), Error> {
        let interactions = documents
            .iter()
            .map(|id| UserInteractionData {
                document_id: id.clone(),
                interaction_type: UserInteractionType::Positive,
            })
            .collect_vec();

        update_interactions(&self.storage, &self.coi, user, &interactions)
            .await
            .map_err(Into::into)
    }

    async fn personalize(
        &self,
        user: &UserId,
        by: PersonalizeBy<'_>,
    ) -> Result<Vec<DocumentId>, Error> {
        personalize_documents_by(&self.storage, &self.coi, user, &self.personalization, by)
            .await
            .map(|documents| documents.into_iter().map(|document| document.id).collect())
            .map_err(Into::into)
    }
}

#[derive(Debug, Deserialize)]
struct ViewedDocument {
    document_id: DocumentId,
    was_clicked: bool,
}

fn deserialize_viewed_documents<'de, D>(deserializer: D) -> Result<Vec<ViewedDocument>, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .split(' ')
        .map(|viewed_document| {
            viewed_document
                .split_once('-')
                .ok_or_else(|| de::Error::custom("missing document id"))
                .and_then(|(document_id, was_clicked)| {
                    let document_id = DocumentId::new(document_id).map_err(de::Error::custom)?;
                    let was_clicked = match was_clicked {
                        "0" => Ok(false),
                        "1" => Ok(true),
                        _ => Err(de::Error::custom("invalid was_clicked")),
                    }?;
                    Ok(ViewedDocument {
                        document_id,
                        was_clicked,
                    })
                })
        })
        .collect()
}

fn deserialize_clicked_documents<'de, D>(deserializer: D) -> Result<Vec<DocumentId>, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .split(' ')
        .map(|document| DocumentId::new(document).map_err(de::Error::custom))
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
    documents: HashMap<DocumentId, Document>,
}

impl DocumentProvider {
    fn new(path: &str) -> Result<Self, Error> {
        let documents = read::<Document>(path)?
            .map(|document| document.map(|document| (document.id.clone(), document)))
            .try_collect()?;
        Ok(Self { documents })
    }

    fn sample(&self, n: usize) -> Vec<&Document> {
        self.documents
            .values()
            .choose_multiple(&mut thread_rng(), n)
    }

    fn get(&self, id: &DocumentId) -> Option<&Document> {
        self.documents.get(id)
    }
}

struct SnippetLabelPair(String, bool);

fn read<T>(path: &str) -> Result<DeserializeRecordsIntoIter<File, T>, Error>
where
    for<'de> T: Deserialize<'de>,
{
    Ok(read_from_tsv(path)?.into_deserialize())
}

fn read_from_tsv<P>(path: P) -> Result<Reader<File>, Error>
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
fn run_benchmark() -> Result<(), Error> {
    let document_provider = DocumentProvider::new("news.tsv")?;

    let impressions = read("behaviors.tsv")?;

    let nranks = vec![3];
    let mut ndcgs = Array::zeros((nranks.len(), 0));

    // Loop over all impressions, prepare reranker with news in click history
    // and rerank the news in an impression
    for impression in impressions {
        let impression: Impression = impression?;

        // Placeholder for interacting with the entire click history
        for click in impression.clicks {
            match document_provider.get(&click) {
                Some(document) => println!("The document {:?} was interacted.", document),
                None => println!("Document id {} not found.", click),
            }
        }

        // Placeholder for reranking the results
        let mut snippet_label_pairs = impression
            .news
            .iter()
            .filter_map(|viewed_document| {
                document_provider
                    .get(&viewed_document.document_id)
                    .map(|document| {
                        SnippetLabelPair(document.snippet.clone(), viewed_document.was_clicked)
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
