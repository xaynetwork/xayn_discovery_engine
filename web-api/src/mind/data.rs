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

use std::{collections::HashMap, fs::File, io, io::Write, path::Path};

use anyhow::Error;
use csv::{DeserializeRecordsIntoIter, Reader, ReaderBuilder};
use derive_more::Deref;
use itertools::Itertools;
use ndarray::{ArrayBase, Data, Dimension};
use npyz::{AutoSerialize, WriterBuilder};
use rand::{rngs::StdRng, seq::IteratorRandom, SeedableRng};
use serde::{de, Deserialize, Deserializer};

use crate::models::{DocumentId, DocumentTag, UserId};

pub(super) fn deserialize_clicked_documents<'de, D>(
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

#[derive(Debug, Deserialize)]
pub(super) struct ViewedDocument {
    pub(super) document_id: DocumentId,
    pub(super) was_clicked: bool,
}

pub(super) fn deserialize_viewed_documents<'de, D>(
    deserializer: D,
) -> Result<Vec<ViewedDocument>, D::Error>
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

#[derive(Debug, Deserialize)]
pub(super) struct Impression {
    #[allow(dead_code)]
    id: String,
    pub(super) user_id: String,
    #[allow(dead_code)]
    time: String,
    #[serde(deserialize_with = "deserialize_clicked_documents")]
    pub(super) clicks: Option<Vec<DocumentId>>,
    #[serde(deserialize_with = "deserialize_viewed_documents")]
    pub(super) news: Vec<ViewedDocument>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct Document {
    pub(super) id: DocumentId,
    pub(super) category: DocumentTag,
    pub(super) subcategory: DocumentTag,
    #[allow(dead_code)]
    title: String,
    pub(super) snippet: String,
    #[allow(dead_code)]
    url: String,
}

impl Document {
    /// Checks if the document is of interest to the user.
    pub(super) fn is_interesting(&self, user_interests: &[String]) -> bool {
        user_interests.iter().any(|interest| {
            let (main_category, sub_category) = interest.split_once('/').unwrap();
            self.category.as_ref() == main_category || self.subcategory.as_ref() == sub_category
        })
    }

    /// Checks if only the main category is matching user's interests
    pub(super) fn is_semi_interesting(&self, user_interests: &[String]) -> bool {
        user_interests.iter().any(|interest| {
            let (main_category, sub_category) = interest.split_once('/').unwrap();
            self.category.as_ref() == main_category || self.subcategory.as_ref() != sub_category
        })
    }
}

/// Assigns a score to a vector of documents based on the user's interests.
///
/// The score is equal to 2 if the document is of interest to the user, 0 otherwise.
/// if the flag is set to true, the score is equal to 1 if the document is semi interesting to the user, 0 otherwise
pub(super) fn score_documents(
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

pub(super) fn read<T>(path: &str) -> Result<DeserializeRecordsIntoIter<File, T>, Error>
where
    for<'de> T: Deserialize<'de>,
{
    Ok(read_from_tsv(path)?.into_deserialize())
}

#[derive(Debug, Deserialize)]
pub(super) struct DocumentProvider {
    pub(super) documents: HashMap<DocumentId, Document>,
}

impl DocumentProvider {
    pub(super) fn new(path: &str) -> Result<Self, Error> {
        let documents = read::<Document>(path)?
            .map(|document| document.map(|document| (document.id.clone(), document)))
            .try_collect()?;
        Ok(Self { documents })
    }

    pub(super) fn sample(&self, n: usize) -> Vec<&Document> {
        self.documents
            .values()
            .choose_multiple(&mut StdRng::seed_from_u64(42), n)
    }

    pub(super) fn get(&self, id: &DocumentId) -> Option<&Document> {
        self.documents.get(id)
    }

    pub(super) fn to_documents(&self) -> Vec<Document> {
        self.documents.values().cloned().collect()
    }

    /// Gets all documents that matches user's interest.
    pub(super) fn get_all_interest(&self, interests: &[String]) -> Vec<&Document> {
        self.documents
            .values()
            .filter(|doc| doc.is_interesting(interests))
            .collect()
    }
}

#[derive(Debug, Deref, Deserialize)]
pub(super) struct SpecificTopics(Vec<String>);

impl SpecificTopics {
    pub(super) fn new(path: &str) -> Result<Self, Error> {
        let file = File::open(path)?;
        let topics = serde_json::from_reader::<_, Vec<String>>(file)?;
        Ok(Self(topics))
    }
}

#[derive(Debug, Deref, Deserialize)]
pub(super) struct Users(HashMap<UserId, Vec<String>>);

impl Users {
    /// Reads the users interests from a json file.
    pub(super) fn new(path: &str) -> Result<Self, Error> {
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

pub(super) fn write_array<T, S, D>(writer: impl Write, array: &ArrayBase<S, D>) -> io::Result<()>
where
    T: Clone + AutoSerialize,
    S: Data<Elem = T>,
    D: Dimension,
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
