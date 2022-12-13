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

use actix_web::{
    body::EitherBody,
    test::TestRequest,
    web::{Data, Json, Query},
    Responder,
};
use anyhow::{bail, Error};
use csv::{DeserializeRecordsIntoIter, Reader, ReaderBuilder};
use itertools::Itertools;
use ndarray::{Array, ArrayView};
use once_cell::sync::Lazy;
use rand::{
    seq::{IteratorRandom, SliceRandom},
    thread_rng,
};
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use xayn_ai_coi::{nan_safe_f32_cmp_desc, CoiConfig, CoiSystem};

use crate::{
    embedding::{self, Embedder},
    ingestion::{self, routes::IngestionRequestBody},
    models::{
        DocumentId,
        DocumentProperties,
        DocumentProperty,
        DocumentPropertyId,
        IngestedDocument,
        UserInteractionType,
    },
    personalization::{
        self,
        routes::{
            PersonalizedDocumentsQuery,
            PersonalizedDocumentsResponse,
            UpdateInteractions,
            UserInteractionData,
        },
    },
    server,
    storage::memory::Storage,
};

pub(crate) struct AppStateExtension {
    pub(crate) embedder: Embedder,
    pub(crate) coi: CoiSystem,
}

#[derive(Default, Deserialize, Serialize)]
pub(crate) struct ConfigExtension {
    #[serde(default)]
    pub(crate) ingestion: ingestion::Config,
    #[serde(default)]
    pub(crate) personalization: personalization::Config,
    #[serde(default)]
    embedding: embedding::Config,
    #[serde(default)]
    coi: CoiConfig,
}

pub(crate) type AppState = server::AppState<ConfigExtension, AppStateExtension, Storage>;

impl AppState {
    fn new() -> Result<Data<Self>, Error> {
        let config = server::Config::<ConfigExtension> {
            extension: ConfigExtension {
                embedding: embedding::Config {
                    directory: "../assets/smbert_v0003".into(),
                    ..embedding::Config::default()
                },
                ..ConfigExtension::default()
            },
            ..server::Config::default()
        };
        let extension = AppStateExtension {
            embedder: Embedder::load(&config.extension.embedding)?,
            coi: config.extension.coi.clone().build(),
        };
        let storage = Storage::default();

        Ok(Data::new(Self {
            config,
            extension,
            storage,
        }))
    }
}

static CATEGORY_ID: Lazy<DocumentPropertyId> = Lazy::new(|| "category".try_into().unwrap());
static SUBCATEGORY_ID: Lazy<DocumentPropertyId> = Lazy::new(|| "subcategory".try_into().unwrap());
static TITLE_ID: Lazy<DocumentPropertyId> = Lazy::new(|| "title".try_into().unwrap());
static SNIPPET_ID: Lazy<DocumentPropertyId> = Lazy::new(|| "snippet".try_into().unwrap());
static URL_ID: Lazy<DocumentPropertyId> = Lazy::new(|| "url".try_into().unwrap());

impl IngestionRequestBody {
    fn new(documents: Vec<Document>) -> Json<Self> {
        let documents = documents
            .into_iter()
            .map(|document| {
                let snippet = if document.snippet.is_empty() {
                    document.title.to_string()
                } else {
                    document.snippet.to_string()
                };
                let category = if document.category.is_empty() {
                    if document.subcategory.is_empty() {
                        None
                    } else {
                        Some(document.subcategory.to_string())
                    }
                } else {
                    Some(document.category.to_string())
                };
                let properties = [
                    (
                        CATEGORY_ID.clone(),
                        DocumentProperty(Value::String(document.category)),
                    ),
                    (
                        SUBCATEGORY_ID.clone(),
                        DocumentProperty(Value::String(document.subcategory)),
                    ),
                    (
                        TITLE_ID.clone(),
                        DocumentProperty(Value::String(document.title)),
                    ),
                    (
                        SNIPPET_ID.clone(),
                        DocumentProperty(Value::String(document.snippet)),
                    ),
                    (
                        URL_ID.clone(),
                        DocumentProperty(Value::String(document.url)),
                    ),
                ]
                .into();

                IngestedDocument {
                    id: document.id,
                    snippet,
                    properties,
                    category,
                }
            })
            .collect();

        Json(Self { documents })
    }
}

impl UpdateInteractions {
    fn new(ids: &[DocumentId]) -> Json<Self> {
        let documents = ids
            .iter()
            .map(|id| UserInteractionData {
                document_id: id.clone(),
                interaction_type: UserInteractionType::Positive,
            })
            .collect();

        Json(Self { documents })
    }
}

impl PersonalizedDocumentsQuery {
    fn new(count: Option<usize>, documents: Option<&[DocumentId]>) -> Query<Self> {
        Query(Self {
            count,
            documents: documents.map(<[DocumentId]>::to_vec),
        })
    }
}

trait PersonalizedDocumentsResponder
where
    Self: Responder<Body = EitherBody<String>> + Sized,
{
    fn extract(self) -> Result<Vec<Document>, Error> {
        match self
            .respond_to(&TestRequest::default().to_http_request())
            .into_body()
        {
            EitherBody::Left { body } => {
                serde_json::from_str::<PersonalizedDocumentsResponse>(&body)
                    .map(|documents| {
                        documents
                            .documents
                            .into_iter()
                            .map(|mut document| {
                                let category =
                                    Self::remove_property(&mut document.properties, &CATEGORY_ID)
                                        .or(document.category)
                                        .unwrap_or_default();
                                let subcategory = Self::remove_property(
                                    &mut document.properties,
                                    &SUBCATEGORY_ID,
                                )
                                .unwrap_or_default();
                                let title =
                                    Self::remove_property(&mut document.properties, &TITLE_ID)
                                        .unwrap_or_default();
                                let snippet =
                                    Self::remove_property(&mut document.properties, &SNIPPET_ID)
                                        .unwrap_or_default();
                                let url = Self::remove_property(&mut document.properties, &URL_ID)
                                    .unwrap_or_default();

                                Document {
                                    id: document.id,
                                    category,
                                    subcategory,
                                    title,
                                    snippet,
                                    url,
                                }
                            })
                            .collect()
                    })
                    .map_err(Into::into)
            }
            EitherBody::Right { body } => bail!("{body:?}"),
        }
    }

    fn remove_property(
        properties: &mut DocumentProperties,
        id: &DocumentPropertyId,
    ) -> Option<String> {
        properties.remove(id).and_then(|property| {
            if let Value::String(property) = property.0 {
                Some(property)
            } else {
                None
            }
        })
    }
}

impl<T> PersonalizedDocumentsResponder for T where T: Responder<Body = EitherBody<String>> {}

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
