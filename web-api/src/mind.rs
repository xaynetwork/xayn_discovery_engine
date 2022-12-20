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
use ndarray::{Array, Array3, ArrayView};
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

// struct that represents config of hyperparameters for the persona based benchmark
#[derive(Debug, Deserialize)]
struct PersonaBasedConfig {
    click_probability: f32,
    n_documents: usize,
    iterations: usize,
    amount_of_doc_used_to_prepare: usize,
    nranks: Vec<usize>,
}

impl Default for PersonaBasedConfig {
    fn default() -> Self {
        Self {
            click_probability: 0.5,
            n_documents: 100,
            iterations: 10,
            amount_of_doc_used_to_prepare: 1,
            nranks: vec![3, 5],
        }
    }
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
    fn is_interesting(&self, user_interests: &[String]) -> bool {
        user_interests.iter().any(|interest| {
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

    fn get_documents(&self) -> Vec<Document> {
        self.documents.values().cloned().collect()
    }

    // get all documents that matches user's interest
    fn get_all_interest(&self, interests: &[String]) -> Vec<&Document> {
        self.documents
            .values()
            .filter(|doc| doc.is_interesting(interests))
            .collect()
    }
}

#[derive(Debug, derive_more::Deref, Deserialize)]
struct Users(HashMap<UserId, Vec<String>>);

impl Users {
    // function that reads the users interests from a json file
    fn new(path: &str) -> Result<Self, Error> {
        let file = File::open(path)?;
        let json = serde_json::from_reader::<_, serde_json::Value>(file)?;
        let map = json.as_object().unwrap();
        // iterate over map and create a map of user ids and their interests
        Ok(Users(
            map.iter()
                .map(|(user_id, interests)| {
                    let interests = interests
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|interest| interest.as_str().unwrap().to_string())
                        .collect();
                    (UserId::new(user_id).unwrap(), interests)
                })
                .collect(),
        ))
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
// score is equal to 2 if the document is interesting to the user, 0 otherwise
fn score_documents(documents: &[&Document], user_interests: &[String]) -> Vec<f32> {
    documents
        .iter()
        .map(|document| {
            if document.is_interesting(user_interests) {
                2.0
            } else {
                0.0
            }
        })
        .collect_vec()
}

fn write_array<T, S, D>(writer: impl io::Write, array: &ndarray::ArrayBase<S, D>) -> io::Result<()>
where
    T: Clone + npyz::AutoSerialize,
    S: ndarray::Data<Elem = T>,
    D: ndarray::Dimension,
{
    let shape = array.shape().iter().map(|&x| x as u64).collect_vec();
    let c_order_items = array.iter();

    let mut writer = npyz::WriteOptions::new()
        .default_dtype()
        .shape(&shape)
        .writer(writer)
        .begin_nd()?;
    writer.extend(c_order_items)?;
    writer.finish()
}
/// # Panics
///
/// The function panics if the provided filenames are not correct.
#[tokio::test]
#[ignore]
async fn run_persona_benchmark() -> Result<(), Error> {
    let document_provider = DocumentProvider::new("news.tsv")?;
    let users_interests = Users::new("user_categories.json")?;
    let state = State::new(Storage::default()).unwrap();
    // load documents from document provider to state
    state
        .insert(document_provider.documents.values().cloned().collect_vec())
        .await
        .unwrap();
    let benchmark_config = PersonaBasedConfig::default();
    // create 3d array of zeros with shape (users, iterations, nranks)
    let mut results = Array3::<f32>::zeros([
        users_interests.len(),
        benchmark_config.iterations,
        benchmark_config.nranks.len(),
    ]);

    let mut rng = thread_rng();

    for (idx, (user_id, interests)) in users_interests.iter().enumerate() {
        let interesting_documents = document_provider.get_all_interest(interests);
        let ids_of_documents_to_prepare = interesting_documents
            .choose_multiple(&mut rng, benchmark_config.amount_of_doc_used_to_prepare)
            .map(|doc| doc.id.clone())
            .collect_vec();
        // prepare reranker by interacting with documents to prepare
        state
            .interact(user_id, &ids_of_documents_to_prepare)
            .await
            .unwrap();

        for iter in 0..benchmark_config.iterations {
            let personalised_documents = state
                .personalize(
                    user_id,
                    PersonalizeBy::KnnSearch(benchmark_config.n_documents),
                )
                .await
                .unwrap();

            let documents = personalised_documents
                .iter()
                .map(|id| document_provider.get(id).unwrap())
                .collect_vec();

            let scores = score_documents(&documents, interests);
            let ndcgs_iteration = ndcg(&scores, &benchmark_config.nranks);
            //save scores to results array
            for (i, ndcg) in ndcgs_iteration.iter().enumerate() {
                results[[idx, iter, i]] = *ndcg;
            }
            // interact with documents
            state
                .interact(
                    user_id,
                    &personalised_documents
                        .iter()
                        .zip(scores.iter())
                        .filter(|(_, &score)| {
                            (score - 2.0).abs() < 0.001
                                || rng.gen_range(0.0..1.0) < benchmark_config.click_probability
                        })
                        .map(|(id, _)| id.clone())
                        .collect_vec(),
                )
                .await
                .unwrap();
        }
    }
    let mut file = File::create("results/persona_based_benchmark_results.npy")?;
    write_array(&mut file, &results)?;
    Ok(())
}

/// Runs the user-based mind benchmark
#[test]
#[ignore]
fn run_user_benchmark() -> Result<(), Error> {
    let document_provider = DocumentProvider::new("news.tsv")?;
    let impressions: DeserializeRecordsIntoIter<File, _> = read("behaviors.tsv")?;

    let state = State::new(Storage::default()).unwrap();
    state
        .insert(document_provider.get_documents())
        .await
        .unwrap();

    let nranks = vec![3];
    let mut ndcgs = Array::zeros((nranks.len(), 0));

    // Loop over all impressions, prepare reranker with news in click history
    // and rerank the news in an impression
    for impression in impressions {
        let impression: Impression = impression?;
        let user = UserId::new(impression.user_id).unwrap();

        state.interact(&user, &impression.clicks).await.unwrap();

        let document_ids = impression
            .news
            .iter()
            .map(|document| &document.document_id)
            .collect::<Vec<_>>();

        let labels = state
            .personalize(&user, PersonalizeBy::Documents(document_ids.as_slice()))
            .await
            .unwrap()
            .iter()
            .map(|reranked_id| {
                impression.news[document_ids
                    .iter()
                    .position(|&actual_id| actual_id == reranked_id)
                    .unwrap()]
                .was_clicked as i32 as f32
            })
            .collect_vec();

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
