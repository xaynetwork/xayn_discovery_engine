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

//! Executes the MIND benchmarks.

use std::{collections::HashMap, fs::File, io, io::Write, path::Path};

use anyhow::Error;
use chrono::{DateTime, Duration, Utc};
use csv::{DeserializeRecordsIntoIter, Reader, ReaderBuilder};
use itertools::Itertools;
use ndarray::{Array, Array3, Array4, ArrayView};
use npyz::WriterBuilder;
use rand::{
    rngs::StdRng,
    seq::{IteratorRandom, SliceRandom},
    Rng,
    SeedableRng,
};
use serde::{de, Deserialize, Deserializer, Serialize};
use xayn_ai_bert::NormalizedEmbedding;
use xayn_ai_coi::{nan_safe_f32_cmp_desc, CoiConfig, CoiSystem};

use crate::{
    embedding::{self, Embedder},
    models::{
        DocumentId,
        DocumentProperties,
        DocumentTag,
        IngestedDocument,
        UserId,
        UserInteractionType,
    },
    personalization::{
        routes::{personalize_documents_by, update_interactions, PersonalizeBy},
        PersonalizationConfig,
    },
    storage::{self, memory::Storage},
};

struct State {
    storage: Storage,
    embedder: Embedder,
    coi: CoiSystem,
    personalization: PersonalizationConfig,
    time: DateTime<Utc>,
}

impl State {
    fn new(storage: Storage, config: StateConfig) -> Result<Self, Error> {
        let embedder = Embedder::load(&embedding::Config {
            directory: "../assets/smbert_v0003".into(),
            ..embedding::Config::default()
        })?;

        let coi = config.coi.build();
        let personalization = config.personalization;
        let time = config.time;

        Ok(Self {
            storage,
            embedder,
            coi,
            personalization,
            time,
        })
    }

    fn with_coi_config(&mut self, config: CoiConfig) {
        self.coi = config.build();
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

    #[allow(dead_code)]
    async fn update(
        &self,
        embeddings: Vec<(DocumentId, NormalizedEmbedding)>,
    ) -> Result<(), Error> {
        let mut documents =
            storage::Document::get_personalized(&self.storage, embeddings.iter().map(|(id, _)| id))
                .await?
                .into_iter()
                .map(|document| (document.id.clone(), document))
                .collect::<HashMap<_, _>>();
        let documents = embeddings
            .into_iter()
            .map(|(id, embedding)| {
                let document = documents.remove(&id).unwrap(/* document must already exist */);
                let document = IngestedDocument {
                    id,
                    snippet: String::new(/* unused for in-memory db */),
                    properties: document.properties,
                    tags: document.tags,
                };
                (document, embedding)
            })
            .collect_vec();
        storage::Document::insert(&self.storage, documents).await?;

        Ok(())
    }

    async fn interact(
        &self,
        user: &UserId,
        documents: impl IntoIterator<Item = (&DocumentId, DateTime<Utc>)>,
    ) -> Result<(), Error> {
        for (id, time) in documents {
            update_interactions(
                &self.storage,
                &self.coi,
                user,
                [&(id.clone(), UserInteractionType::Positive)],
                self.personalization.store_user_history,
                time,
            )
            .await?;
        }

        Ok(())
    }

    async fn personalize(
        &self,
        user: &UserId,
        by: PersonalizeBy<'_>,
        time: DateTime<Utc>,
    ) -> Result<Option<Vec<DocumentId>>, Error> {
        personalize_documents_by(
            &self.storage,
            &self.coi,
            user,
            &self.personalization,
            by,
            time,
        )
        .await
        .map(|documents| {
            documents.map(|documents| documents.into_iter().map(|document| document.id).collect())
        })
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

fn deserialize_clicked_documents<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<DocumentId>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?
        .as_ref()
        .map(|m| {
            m.split(' ')
                .map(|document| DocumentId::new(document).map_err(de::Error::custom))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()
}

struct GridSearchConfig {
    thresholds: Vec<f32>,
    shifts: Vec<f32>,
    min_pos_cois: Vec<usize>,
    click_probability: f64,
    ndocuments: usize,
    iterations: usize,
    nranks: Vec<usize>,
    is_semi_interesting: bool,
}

impl Default for GridSearchConfig {
    fn default() -> Self {
        Self {
            thresholds: vec![0.67, 0.7, 0.75, 0.8, 0.85, 0.9],
            shifts: vec![0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4],
            min_pos_cois: vec![1],
            click_probability: 0.2,
            ndocuments: 100,
            iterations: 10,
            nranks: vec![3, 5],
            is_semi_interesting: false,
        }
    }
}

/// The config of hyperparameters for the persona based benchmark.
#[derive(Debug, Deserialize)]
struct PersonaBasedConfig {
    click_probability: f64,
    ndocuments: usize,
    iterations: usize,
    amount_of_doc_used_to_prepare: usize,
    nranks: Vec<usize>,
    ndocuments_hot_news: usize,
    is_semi_interesting: bool,
}

impl Default for PersonaBasedConfig {
    fn default() -> Self {
        Self {
            click_probability: 0.2,
            ndocuments: 100,
            iterations: 10,
            amount_of_doc_used_to_prepare: 1,
            nranks: vec![3, 5],
            ndocuments_hot_news: 15,
            is_semi_interesting: false,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SaturationConfig {
    click_probability: f64,
    ndocuments: usize,
    iterations: usize,
}

impl Default for SaturationConfig {
    fn default() -> Self {
        Self {
            click_probability: 0.2,
            ndocuments: 30,
            iterations: 10,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct StateConfig {
    coi: CoiConfig,
    personalization: PersonalizationConfig,
    time: DateTime<Utc>,
}

impl Default for StateConfig {
    fn default() -> Self {
        Self {
            coi: CoiConfig::default(),
            personalization: PersonalizationConfig::default(),
            time: Utc::now(),
        }
    }
}

/// The results of iteration of the saturation benchmark
#[derive(Debug, Default, Serialize)]
struct SaturationIteration {
    shown_documents: Vec<DocumentId>,
    clicked_documents: Vec<DocumentId>,
}

/// The results of the saturation benchmark for one topic
#[derive(Debug, Default, Serialize)]
struct SaturationTopicResult {
    topic: String,
    iterations: Vec<SaturationIteration>,
}

impl SaturationTopicResult {
    fn new(topic: &str, iterations: usize) -> Self {
        Self {
            topic: topic.to_owned(),
            iterations: Vec::with_capacity(iterations),
        }
    }
}

/// The results of the saturation benchmark
#[derive(Debug, Default, Serialize)]
struct SaturationResult {
    topics: Vec<SaturationTopicResult>,
}

impl SaturationResult {
    fn new(topics: usize) -> Self {
        Self {
            topics: Vec::with_capacity(topics),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Impression {
    #[allow(dead_code)]
    id: String,
    user_id: String,
    #[allow(dead_code)]
    time: String,
    #[serde(deserialize_with = "deserialize_clicked_documents")]
    clicks: Option<Vec<DocumentId>>,
    #[serde(deserialize_with = "deserialize_viewed_documents")]
    news: Vec<ViewedDocument>,
}

#[derive(Clone, Debug, Deserialize)]
struct Document {
    id: DocumentId,
    category: DocumentTag,
    subcategory: DocumentTag,
    #[allow(dead_code)]
    title: String,
    snippet: String,
    #[allow(dead_code)]
    url: String,
}

impl Document {
    /// Checks if the document is of interest to the user.
    fn is_interesting(&self, user_interests: &[String]) -> bool {
        user_interests.iter().any(|interest| {
            let (main_category, sub_category) = interest.split_once('/').unwrap();
            self.category.as_ref() == main_category || self.subcategory.as_ref() == sub_category
        })
    }

    /// Checks if only the main category is matching user's interests
    fn is_semi_interesting(&self, user_interests: &[String]) -> bool {
        user_interests.iter().any(|interest| {
            let (main_category, sub_category) = interest.split_once('/').unwrap();
            self.category.as_ref() == main_category || self.subcategory.as_ref() != sub_category
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

    #[allow(dead_code)]
    fn sample(&self, n: usize) -> Vec<&Document> {
        self.documents
            .values()
            .choose_multiple(&mut StdRng::seed_from_u64(42), n)
    }

    fn get(&self, id: &DocumentId) -> Option<&Document> {
        self.documents.get(id)
    }

    fn to_documents(&self) -> Vec<Document> {
        self.documents.values().cloned().collect()
    }

    /// Gets all documents that matches user's interest.
    fn get_all_interest(&self, interests: &[String]) -> Vec<&Document> {
        self.documents
            .values()
            .filter(|doc| doc.is_interesting(interests))
            .collect()
    }
}
#[derive(Debug, derive_more::Deref, Deserialize)]
struct SpecificTopics(Vec<String>);

impl SpecificTopics {
    fn new(path: &str) -> Result<Self, Error> {
        let file = File::open(path)?;
        let topics = serde_json::from_reader::<_, Vec<String>>(file)?;
        Ok(Self(topics))
    }
}

#[derive(Debug, derive_more::Deref, Deserialize)]
struct Users(HashMap<UserId, Vec<String>>);

impl Users {
    /// Reads the users interests from a json file.
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
}

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
        .flexible(true)
        .from_path(path)
        .map_err(Into::into)
}

/// Assigns a score to a vector of documents based on the user's interests.
///
/// The score is equal to 2 if the document is of interest to the user, 0 otherwise.
/// if the flag is set to true, the score is equal to 1 if the document is semi interesting to the user, 0 otherwise
fn score_documents(
    documents: &[&Document],
    user_interests: &[String],
    is_semi_interesting: bool,
) -> Vec<f32> {
    documents
        .iter()
        .map(|document| {
            if document.is_interesting(user_interests) {
                2.0
            } else if is_semi_interesting && document.is_semi_interesting(user_interests) {
                1.0
            } else {
                0.0
            }
        })
        .collect_vec()
}

fn write_array<T, S, D>(writer: impl Write, array: &ndarray::ArrayBase<S, D>) -> io::Result<()>
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

fn create_grid_search_configs(grid_search_config: &GridSearchConfig) -> Vec<StateConfig> {
    let mut configs = Vec::with_capacity(
        grid_search_config.thresholds.len()
            * grid_search_config.shifts.len()
            * grid_search_config.min_pos_cois.len(),
    );

    let start_time = Utc::now();

    for t in &grid_search_config.thresholds {
        for s in &grid_search_config.shifts {
            for m in &grid_search_config.min_pos_cois {
                configs.push(StateConfig {
                    coi: {
                        CoiConfig::default()
                            .with_shift_factor(*s)
                            .unwrap()
                            .with_threshold(*t)
                            .unwrap()
                            .with_min_positive_cois(*m)
                            .unwrap()
                    },
                    personalization: PersonalizationConfig::default(),
                    time: start_time,
                });
            }
        }
    }
    configs
}

/// Runs the persona-based mind benchmark.
#[tokio::test]
#[ignore = "run on demand via `just mind-benchmark persona`"]
async fn run_persona_benchmark() -> Result<(), Error> {
    let users_interests = Users::new("user_categories.json")?;
    let document_provider = DocumentProvider::new("news.tsv")?;

    let state = State::new(Storage::default(), StateConfig::default()).unwrap();
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

    let mut rng = StdRng::seed_from_u64(42);
    for (idx, (user_id, interests)) in users_interests.iter().enumerate() {
        let interesting_documents = document_provider.get_all_interest(interests);
        // prepare reranker by interacting with documents to prepare
        state
            .interact(
                user_id,
                interesting_documents
                    .choose_multiple(&mut rng, benchmark_config.amount_of_doc_used_to_prepare)
                    .map(|doc| {
                        (
                            &doc.id,
                            // TODO: set some meaningful value for the interaction time
                            state.time - Duration::days(0),
                        )
                    }),
            )
            .await
            .unwrap();

        for iter in 0..benchmark_config.iterations {
            let personalised_documents = state
                .personalize(
                    user_id,
                    PersonalizeBy::KnnSearch {
                        count: benchmark_config.ndocuments,
                        published_after: None,
                    },
                    state.time,
                )
                .await
                .unwrap()
                .unwrap();

            let documents = personalised_documents
                .iter()
                .map(|id| document_provider.get(id).unwrap())
                .collect_vec();

            let scores =
                score_documents(&documents, interests, benchmark_config.is_semi_interesting);
            let ndcgs_iteration = ndcg(&scores, &benchmark_config.nranks);
            //save scores to results array
            for (i, ndcg) in ndcgs_iteration.iter().enumerate() {
                results[[idx, iter, i]] = *ndcg;
            }
            // interact with documents
            state
                .interact(
                    user_id,
                    personalised_documents
                        .iter()
                        .zip(scores.iter())
                        .filter(|(_, &score)| {
                            (score - 2.0).abs() < 0.001
                                && rng.gen_bool(benchmark_config.click_probability)
                        })
                        .map(|(id, _)| {
                            (
                                id,
                                // TODO: set some meaningful value for the interaction time
                                state.time - Duration::days(0),
                            )
                        }),
                )
                .await
                .unwrap();
        }
    }
    let mut file = File::create("results/persona_based_benchmark_results.npy")?;
    write_array(&mut file, &results)?;
    Ok(())
}

/// Runs the user-based mind benchmark.
#[tokio::test]
#[ignore = "run on demand via `just mind-benchmark user`"]
async fn run_user_benchmark() -> Result<(), Error> {
    let document_provider = DocumentProvider::new("news.tsv")?;

    let state = State::new(Storage::default(), StateConfig::default()).unwrap();
    state
        .insert(document_provider.to_documents())
        .await
        .unwrap();

    let nranks = vec![5, 10];
    let mut ndcgs = Array::zeros((nranks.len(), 0));
    let mut users = Vec::new();

    // Loop over all impressions, prepare reranker with news in click history
    // and rerank the news in an impression
    for impression in read::<Impression>("behaviors.tsv")? {
        let impression = impression?;
        let labels = if let Some(clicks) = impression.clicks {
            let user = UserId::new(&impression.user_id).unwrap();
            if !users.contains(&impression.user_id) {
                users.push(impression.user_id);
                state
                    .interact(
                        &user,
                        clicks.iter().map(|document| {
                            (
                                document,
                                // TODO: set some meaningful value for the interaction time
                                state.time - Duration::days(0),
                            )
                        }),
                    )
                    .await
                    .unwrap();
            }

            let document_ids = impression
                .news
                .iter()
                .map(|document| &document.document_id)
                .collect::<Vec<_>>();

            state
                .personalize(
                    &user,
                    PersonalizeBy::Documents(document_ids.as_slice()),
                    state.time,
                )
                .await
                .unwrap()
                .unwrap()
                .iter()
                .map(|reranked_id| {
                    u8::from(
                        impression.news[document_ids
                            .iter()
                            .position(|&actual_id| actual_id == reranked_id)
                            .unwrap()]
                        .was_clicked,
                    )
                    .into()
                })
                .collect_vec()
        } else {
            impression
                .news
                .iter()
                .map(|viewed_document| u8::from(viewed_document.was_clicked).into())
                .collect_vec()
        };

        let ndcgs_iteration = ndcg(&labels, &nranks);
        ndcgs
            .push_column(ArrayView::from(&ndcgs_iteration))
            .unwrap();
    }
    println!("{ndcgs:?}");

    Ok(())
}

/// The function panics if the provided filenames are not correct.
#[tokio::test]
#[ignore]
async fn run_saturation_benchmark() -> Result<(), Error> {
    // load list of possible specific topics from file (need to create it)
    let specific_topics = SpecificTopics::new("topics.json")?;
    let document_provider = DocumentProvider::new("news.tsv")?;
    let state = State::new(Storage::default(), StateConfig::default()).unwrap();
    // load documents from document provider to state
    state
        .insert(document_provider.documents.values().cloned().collect_vec())
        .await
        .unwrap();
    let benchmark_config = SaturationConfig::default();

    // create rng thread for random number generation with seed 42
    let mut rng = StdRng::seed_from_u64(42);

    // create results preserving structure
    let mut results = SaturationResult::new(specific_topics.len());

    // iterate over specific topics get the documents for each topic and interact with one document
    // from each topic and then run iterations of personalized search
    for full_category in specific_topics.iter() {
        //create saturation topic result structure
        let mut topic_result =
            SaturationTopicResult::new(full_category, benchmark_config.iterations);

        let documents = document_provider.get_all_interest(std::slice::from_ref(full_category));
        let user_id = UserId::new(full_category).unwrap();
        // get random document from the topic
        let document = documents.choose(&mut rng).unwrap();

        // interact with the document
        state
            .interact(&user_id, [(&document.id, state.time - Duration::days(0))])
            .await
            .unwrap();

        for _ in 0..benchmark_config.iterations {
            let personalised_documents = state
                .personalize(
                    &user_id,
                    PersonalizeBy::KnnSearch {
                        count: benchmark_config.ndocuments,
                        published_after: None,
                    },
                    state.time,
                )
                .await
                .unwrap()
                .unwrap();
            // calculate scores for the documents
            let documents = personalised_documents
                .iter()
                .map(|id| document_provider.get(id).unwrap())
                .collect_vec();

            let scores = score_documents(&documents, &[full_category.clone()], false);
            let to_be_clicked = personalised_documents
                .iter()
                .zip(scores)
                .filter_map(|(id, score)| {
                    ((score - 2.0).abs() < 0.001
                        && rng.gen_bool(benchmark_config.click_probability))
                    .then(|| id.clone())
                })
                .collect_vec();
            // interact with documents
            state
                .interact(
                    &user_id,
                    to_be_clicked
                        .iter()
                        .map(|id| (id, state.time - Duration::days(0))),
                )
                .await
                .unwrap();

            // add results to the topic result
            topic_result.iterations.push(SaturationIteration {
                shown_documents: personalised_documents,
                clicked_documents: to_be_clicked,
            });
        }
        results.topics.push(topic_result);
    }
    //save results to json file
    let file = File::create("saturation_results.json")?;
    serde_json::to_writer(file, &results)?;

    Ok(())
}

#[tokio::test]
#[ignore]
async fn run_persona_hot_news_benchmark() -> Result<(), Error> {
    let users_interests = Users::new("user_categories.json")?;
    let document_provider = DocumentProvider::new("news.tsv")?;
    let state = State::new(Storage::default(), StateConfig::default()).unwrap();
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

    let mut rng = StdRng::seed_from_u64(42);

    for (idx, (user_id, interests)) in users_interests.iter().enumerate() {
        let interesting_documents = document_provider.get_all_interest(interests);
        // prepare reranker by interacting with documents to prepare
        state
            .interact(
                user_id,
                interesting_documents
                    .choose_multiple(&mut rng, benchmark_config.amount_of_doc_used_to_prepare)
                    .map(|doc| (&doc.id, state.time - Duration::days(0))),
            )
            .await
            .unwrap();

        for iter in 0..benchmark_config.iterations {
            // get random news that will represent tenant's hot news category
            let hot_news = document_provider.sample(benchmark_config.ndocuments_hot_news); // number of news will be configurable

            // go through hot news and if they are in user's sphere of interests then click them with some probability
            state
                .interact(
                    user_id,
                    hot_news.iter().filter_map(|doc| {
                        (doc.is_interesting(interests)
                            && rng.gen_bool(benchmark_config.click_probability))
                        .then(|| (&doc.id, state.time - Duration::days(0)))
                    }),
                )
                .await
                .unwrap();

            let personalised_documents = state
                .personalize(
                    user_id,
                    PersonalizeBy::KnnSearch {
                        count: benchmark_config.ndocuments,
                        published_after: None,
                    },
                    state.time,
                )
                .await
                .unwrap()
                .unwrap();

            let documents = personalised_documents
                .iter()
                .map(|id| document_provider.get(id).unwrap())
                .collect_vec();

            let scores =
                score_documents(&documents, interests, benchmark_config.is_semi_interesting);
            let ndcgs_iteration = ndcg(&scores, &benchmark_config.nranks);
            //save scores to results array
            results
                .slice_mut(ndarray::s![idx, iter, ..])
                .assign(&Array::from(ndcgs_iteration));
            // interact with documents
            state
                .interact(
                    user_id,
                    personalised_documents
                        .iter()
                        .zip(scores)
                        .filter_map(|(id, score)| {
                            ((score - 2.0).abs() < 0.001
                                && rng.gen_bool(benchmark_config.click_probability))
                            .then(|| (id, state.time - Duration::days(0)))
                        }),
                )
                .await
                .unwrap();
        }
    }
    let mut file = File::create("results/persona_based_benchmark_results.npy")?;
    write_array(&mut file, &results)?;
    Ok(())
}

// grid search for best parameters for persona based benchmark as test
#[tokio::test]
#[ignore]
async fn grid_search_for_best_parameters() -> Result<(), Error> {
    // load users interests sample as computing all users interests is too expensive in grid search
    let users_interests = Users::new("user_categories_sample.json")?;
    let document_provider = DocumentProvider::new("news.tsv")?;
    let grid_search_config = GridSearchConfig::default();
    let configs = create_grid_search_configs(&grid_search_config);
    let mut state = State::new(Storage::default(), StateConfig::default()).unwrap();
    state
        .insert(document_provider.to_documents())
        .await
        .unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let file = File::create("params.json")?;
    let mut writer = io::BufWriter::new(file);
    serde_json::to_writer(&mut writer, &configs)?;
    writer.flush()?;

    let mut ndcgs_all_configs = Array4::zeros((
        configs.len(),
        users_interests.len(),
        grid_search_config.iterations,
        grid_search_config.nranks.len(),
    ));
    for (config_idx, config) in configs.into_iter().enumerate() {
        state.with_coi_config(config.coi);
        for (idx, (user_id, interests)) in users_interests.iter().enumerate() {
            let interesting_documents = document_provider.get_all_interest(interests);
            state
                .interact(
                    user_id,
                    interesting_documents
                        .choose_multiple(&mut rng, state.coi.config().min_positive_cois())
                        .map(|doc| (&doc.id, state.time - Duration::days(0))),
                )
                .await
                .unwrap();

            for iter in 0..grid_search_config.iterations {
                let personalised_documents = state
                    .personalize(
                        user_id,
                        PersonalizeBy::KnnSearch {
                            count: grid_search_config.ndocuments,
                            published_after: None,
                        },
                        state.time,
                    )
                    .await
                    .unwrap()
                    .unwrap();

                let documents = personalised_documents
                    .iter()
                    .map(|id| document_provider.get(id).unwrap())
                    .collect_vec();

                let scores = score_documents(
                    &documents,
                    interests,
                    grid_search_config.is_semi_interesting,
                );
                let ndcgs_iteration = ndcg(&scores, &grid_search_config.nranks);
                //save scores to results array
                ndcgs_all_configs
                    .slice_mut(ndarray::s![config_idx, idx, iter, ..])
                    .assign(&Array::from(ndcgs_iteration));
                // interact with documents
                state
                    .interact(
                        user_id,
                        personalised_documents
                            .iter()
                            .zip(scores)
                            .filter_map(|(id, score)| {
                                ((score - 2.0).abs() < 0.001
                                    && rng.gen_bool(grid_search_config.click_probability))
                                .then_some((id, state.time))
                            }),
                    )
                    .await
                    .unwrap();
            }
        }
    }
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

    let ndcgs = relevance
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
        .collect::<Vec<_>>();

    k.iter()
        .map(|nrank| match ndcgs.get(*nrank - 1) {
            Some(i) => i,
            None => ndcgs.last().unwrap(),
        })
        .copied()
        .collect::<Vec<_>>()
}
