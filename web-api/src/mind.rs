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

use std::{collections::HashMap, fs::File, io, path::Path};

use anyhow::Error;
use csv::{DeserializeRecordsIntoIter, Reader, ReaderBuilder};
use itertools::Itertools;
use ndarray::{Array, ArrayView, Axis};
use npyz::WriterBuilder;
use rand::{
    seq::{IteratorRandom, SliceRandom},
    thread_rng,
    Rng,
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
                let tags = if document.category.is_empty() {
                    if document.subcategory.is_empty() {
                        None
                    } else {
                        Some(document.subcategory)
                    }
                } else {
                    Some(document.category)
                };
                let document = IngestedDocument {
                    id: document.id,
                    snippet: document.snippet,
                    properties: DocumentProperties::default(),
                    tags,
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

impl Document {
    // check if the document is interesting to the user
    fn is_interesting(&self, user_interests: &UserInterests) -> bool {
        user_interests.interests.iter().any(|interest| {
            let (main_category, sub_category) = interest.split_once('/').unwrap();
            self.category == main_category || self.subcategory == sub_category
        })
    }
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

    // get all documents that matches user's interest
    fn get_all_interest(&self, interests: &UserInterests) -> Vec<&Document> {
        self.documents
            .values()
            .filter(|doc| doc.is_interesting(interests))
            .collect()
    }
}

// struct storing users interests. It's a list of user ids and a list of their interests
#[derive(Debug, Deserialize)]
struct UserInterests {
    user_id: String,
    interests: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct UsersInterests {
    user_interests: Vec<UserInterests>,
}

impl UsersInterests {
    // function that reads the users interests from a json file
    fn new(path: &str) -> Result<Self, Error> {
        let file = File::open(path)?;
        let json: serde_json::Value = serde_json::from_reader(file)?;
        let map = json.as_object().unwrap();
        // iterate over map and create a vector of UserInterests
        Ok(UsersInterests {
            user_interests: map
                .into_iter()
                .map(|(user_id, interests)| UserInterests {
                    user_id: user_id.to_string(),
                    interests: interests
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|interest| interest.as_str().unwrap().to_string())
                        .collect(),
                })
                .collect(),
        })
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

// function that assigns a score to a vector of documents based on the user's interests
// score is equal to 1 if the document is interesting to the user, 0 otherwise
fn score_documents(documents: &[&Document], user_interests: &UserInterests) -> Vec<f32> {
    documents
        .iter()
        .map(|document| {
            if document.is_interesting(user_interests) {
                2.0
            } else {
                0.0
            }
        })
        .collect()
}

fn write_array<T, S, D>(writer: impl io::Write, array: &ndarray::ArrayBase<S, D>) -> io::Result<()>
where
    T: Clone + npyz::AutoSerialize,
    S: ndarray::Data<Elem = T>,
    D: ndarray::Dimension,
{
    let shape = array.shape().iter().map(|&x| x as u64).collect::<Vec<_>>();
    let c_order_items = array.iter();

    let mut writer = npyz::WriteOptions::new()
        .default_dtype()
        .shape(&shape)
        .writer(writer)
        .begin_nd()?;
    writer.extend(c_order_items)?;
    writer.finish()
}

/// Runs persona based benchmark
async fn run_persona_based_benchmark() -> Result<(), Error> {
    let document_provider = DocumentProvider::new("news.tsv")?;
    let state = State::new(Storage::default()).unwrap();
    // load documents from document provider to state
    state
        .insert(
            document_provider
                .documents
                .values()
                .cloned()
                .collect::<Vec<_>>(),
        )
        .await
        .unwrap();
    let click_probability = 0.5;
    let n_documents = 100;
    let iterations = 10;
    let amount_of_doc_used_to_prepare = 1;
    let nranks = vec![3, 5];
    let mut results = Array::zeros((nranks.len(), iterations, 0));

    let mut rng = thread_rng();

    let users_interests = UsersInterests::new("user_categories.json")?;
    for user_interest in users_interests.user_interests {
        let user_id = UserId::new(&user_interest.user_id).unwrap();
        let interesting_documents = document_provider.get_all_interest(&user_interest);
        let ids_of_documents_to_prepare = interesting_documents
            .choose_multiple(&mut rng, amount_of_doc_used_to_prepare)
            .map(|doc| doc.id.clone())
            .collect::<Vec<_>>();
        let mut ndcgs = Array::zeros((nranks.len(), 0));
        // prepare reranker by interacting with documents to prepare
        state
            .interact(&user_id, &ids_of_documents_to_prepare)
            .await
            .unwrap();
        // perform iterations
        for _ in 0..iterations {
            // get personalised documents from state
            let personalised_documents = state
                .personalize(&user_id, PersonalizeBy::KnnSearch(n_documents))
                .await
                .unwrap();
            // get documents based on the personalised documents ids
            let documents = personalised_documents
                .iter()
                .map(|id| document_provider.get(id).unwrap())
                .collect::<Vec<_>>();
            // score documents based on the user's interests
            let scores = score_documents(&documents, &user_interest);
            // add scores to user's ndcgs
            ndcgs.push(Axis(1), ArrayView::from(&scores)).unwrap();
            // interact with some of the retrieved documents based on whether the document is interesting to the user and probability of clicking
            // interact with documents
            state
                .interact(
                    &user_id,
                    &personalised_documents
                        .iter()
                        .zip(scores.iter())
                        .filter(|(_, &score)| {
                            (score - 2.0).abs() < 0.001
                                || rng.gen_range(0.0..1.0) < click_probability
                        })
                        .map(|(id, _)| id.clone())
                        .collect::<Vec<_>>(),
                )
                .await
                .unwrap();
        }
        results.push(Axis(2), ArrayView::from(&ndcgs)).unwrap();
    }
    let mut file = File::create("results/persona_based_benchmark_results.txt")?;
    // save results to file
    write_array(&mut file, &results)?;
    Ok(())
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
