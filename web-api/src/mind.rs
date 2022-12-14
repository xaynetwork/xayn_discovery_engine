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
use std::{collections::HashMap, fs::File, io, path::Path};

use csv::{DeserializeRecordsIntoIter, Reader, ReaderBuilder};
use itertools::Itertools;
use ndarray::{Array, ArrayView, Axis};
use npyz::WriterBuilder;
use rand::{
    seq::{IteratorRandom, SliceRandom},
    thread_rng, Rng,
};
use serde::{de, Deserialize, Deserializer};
use xayn_ai_coi::nan_safe_f32_cmp_desc;

use crate::models::DocumentId;

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
            let (main_category, sub_category) = interest.split_once("/").unwrap();
            self.category == main_category || self.subcategory == sub_category
        })
    }
}

#[derive(Debug, Deserialize)]
struct DocumentProvider {
    documents: HashMap<DocumentId, Document>,
}

impl DocumentProvider {
    fn new(path: &str) -> Result<Self, anyhow::Error> {
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
    fn new(path: &str) -> Result<Self, anyhow::Error> {
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

// function that assigns a score to a vector of documents based on the user's interests
// score is equal to 1 if the document is interesting to the user, 0 otherwise
fn score_documents(documents: &Vec<&Document>, user_interests: &UserInterests) -> Vec<f32> {
    documents
        .iter()
        .map(|document| {
            if document.is_interesting(user_interests) {
                1.0
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

/// Runs the persona-based MIND benchmark. We load the user's preferences from the json file and
/// then use the user's preferences to select the documents that are relevant to the user.
/// The documents are then used to prepare reranker. The reranker is then used to rerank the
/// randomly selected documents. Then NDCG is calculated.
fn run_persona_based_benchmark() -> Result<(), anyhow::Error> {
    // define random thread
    let mut rng = thread_rng();
    // define how many documents to sample
    let amount_of_doc_used_to_prepare = 1;

    // define how many documents to rerank
    let amount_of_doc_used_to_rerank = 30;

    //probability of a document being clicked if it is interesting to the user
    let click_probability = 0.5;

    //define how many iterations to perform
    let iterations = 10;

    // define how many documents to use to calculate NDCG
    let nranks = vec![3, 5];

    // create a 3d array to store the results
    let mut results = Array::zeros((nranks.len(), iterations, 0));

    // read in users' interests from json file
    let users_interests = UsersInterests::new("user_categories.json").unwrap();
    // read in documents from tsv file
    let document_provider = DocumentProvider::new("news.tsv").unwrap();
    // iterate over users' interests and select documents that are relevant to the user
    for user_interest in users_interests.user_interests {
        // get all documents that match user's interests
        let interesting_documents = document_provider.get_all_interest(&user_interest);
        // sample documents that will be used to prepare the reranker from interesting documents
        let documents_to_prepare =
            interesting_documents.choose_multiple(&mut rng, amount_of_doc_used_to_prepare);

        let mut ndcgs = Array::zeros((nranks.len(), 0));

        // Placeholder for interacting with the sampled news (preparing the reranker)
        for document in documents_to_prepare {
            println!("The document {:?} was interacted with.", document);
        }

        // running iterations
        for _ in 0..iterations {
            // Sample random documents
            let sampled_documents = document_provider.sample(amount_of_doc_used_to_rerank);
            //Placeholder for reranking the sampled documents
            for document in &sampled_documents {
                println!("The document {:?} was reranked.", document);
            }
            // Assing labels to the documents
            let labels = score_documents(&sampled_documents, &user_interest);
            // Calculate NDCG
            let iteration_ndcg = ndcg(&labels, &nranks);

            ndcgs
                .push(Axis(1), ArrayView::from(&iteration_ndcg))
                .unwrap();

            //placeholder for interacting with the sampled news
            for document in sampled_documents {
                if document.is_interesting(&user_interest) {
                    // user clicked on the document if the probability is greater than click_probability
                    if rng.gen::<f32>() < click_probability {
                        println!("The document {:?} was clicked.", document);
                    }
                }
            }
        }
        // add ndcgs for the user to the results
        results.push(Axis(2), ArrayView::from(&ndcgs)).unwrap();
    }

    // save the results to a npy file
    let mut file = io::BufWriter::new(File::create("ndarray.npy")?);

    write_array(&mut file, &results)?;
    Ok(())
}

/// Runs the user-based mind benchmark
fn run_benchmark() -> Result<(), anyhow::Error> {
    let document_provider = DocumentProvider::new("/Users/maciejkrajewski/CLionProjects/xayn_discovery_engine/web-api/src/bin/news_no_nans.tsv")?;

    let impressions = read(
        "/Users/maciejkrajewski/CLionProjects/xayn_discovery_engine/web-api/src/bin/behaviors.tsv",
    )?;

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
    run_persona_based_benchmark().expect("TODO: panic message");
}
