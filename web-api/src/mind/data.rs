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

use std::{collections::HashMap, fs::File, io::Write};

use csv::{DeserializeRecordsIntoIter, ReaderBuilder};
use derive_more::Deref;
use itertools::Itertools;
use ndarray::{Array, Array1, Dimension, Ix2, ShapeBuilder, SliceArg};
use npyz::WriterBuilder;
use rand::{rngs::StdRng, seq::IteratorRandom, SeedableRng};
use serde::{de, Deserialize, Deserializer};
use xayn_test_utils::error::Panic;

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
                .try_collect::<_, Vec<_>, _>()
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
            *self.category == main_category || *self.subcategory == sub_category
        })
    }

    /// Checks if only the main category is matching user's interests
    pub(super) fn is_semi_interesting(&self, user_interests: &[String]) -> bool {
        user_interests.iter().any(|interest| {
            let (main_category, sub_category) = interest.split_once('/').unwrap();
            *self.category == main_category || *self.subcategory != sub_category
        })
    }
}

pub(super) fn read<T>(path: &str) -> Result<DeserializeRecordsIntoIter<File, T>, Panic>
where
    T: de::DeserializeOwned,
{
    Ok(ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .flexible(true)
        .from_path(path)?
        .into_deserialize())
}

#[derive(Debug, Deref, Deserialize)]
#[serde(transparent)]
pub(super) struct DocumentProvider(HashMap<DocumentId, Document>);

impl DocumentProvider {
    pub(super) fn new(path: &str) -> Result<Self, Panic> {
        read::<Document>(path)?
            .map(|document| document.map(|document| (document.id.clone(), document)))
            .try_collect()
            .map(Self)
            .map_err(Into::into)
    }

    pub(super) fn sample(&self, n: usize) -> Vec<&Document> {
        self.values()
            .choose_multiple(&mut StdRng::seed_from_u64(42), n)
    }

    pub(super) fn to_documents(&self) -> Vec<Document> {
        self.values().cloned().collect()
    }

    /// Gets all documents that matches user's interest.
    pub(super) fn get_all_interest(&self, interests: &[String]) -> Vec<&Document> {
        self.values()
            .filter(|doc| doc.is_interesting(interests))
            .collect()
    }

    /// Assigns a score to documents based on the user's interests.
    ///
    /// The score is equal to 2 if the document is of interest to the user, 0 otherwise. If the flag
    /// is set to true, the score is equal to 1 if the document is semi interesting to the user, 0
    /// otherwise.
    pub(super) fn score(
        &self,
        ids: &[DocumentId],
        user_interests: &[String],
        is_semi_interesting: bool,
    ) -> Vec<f32> {
        ids.iter()
            .map(|id| {
                let document = &self.0[id];
                if document.is_interesting(user_interests) {
                    2.0
                } else if is_semi_interesting && document.is_semi_interesting(user_interests) {
                    1.0
                } else {
                    0.0
                }
            })
            .collect()
    }
}

#[derive(Debug, Deref, Deserialize)]
#[serde(transparent)]
pub(super) struct SpecificTopics(Vec<String>);

impl SpecificTopics {
    pub(super) fn new(path: &str) -> Result<Self, Panic> {
        Ok(serde_json::from_reader(File::open(path)?)?)
    }
}

#[derive(Debug, Deref, Deserialize)]
#[serde(transparent)]
pub(super) struct Users(HashMap<UserId, Vec<String>>);

impl Users {
    /// Reads the users interests from a json file.
    pub(super) fn new(path: &str) -> Result<Self, Panic> {
        Ok(serde_json::from_reader(File::open(path)?)?)
    }
}

#[derive(Debug)]
pub(super) struct Ndcg<D>(Array<f32, D>)
where
    D: Dimension;

impl<D> Ndcg<D>
where
    D: Dimension,
{
    pub(super) fn new(shape: impl ShapeBuilder<Dim = D>) -> Self {
        Self(Array::<f32, _>::zeros(shape))
    }

    fn compute(relevance: &[f32], k: &[usize]) -> Array1<f32> {
        let mut optimal_order = relevance.to_vec();
        optimal_order.sort_by(|r1, r2| r1.total_cmp(r2).reverse());
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
            .collect_vec();

        k.iter()
            .map(|nrank| match ndcgs.get(*nrank - 1) {
                Some(i) => i,
                None => ndcgs.last().unwrap(),
            })
            .copied()
            .collect()
    }

    pub(super) fn assign(&mut self, indices: impl SliceArg<D>, relevance: &[f32], k: &[usize]) {
        self.0
            .slice_mut(indices)
            .assign(&Self::compute(relevance, k));
    }

    pub(super) fn write(&self, writer: impl Write) -> Result<(), Panic> {
        let mut writer = npyz::WriteOptions::new()
            .writer(writer)
            .default_dtype()
            .shape(&self.0.shape().iter().map(|&x| x as u64).collect_vec())
            .begin_nd()?;
        writer.extend(&self.0)?;
        writer.finish()?;

        Ok(())
    }
}

impl Ndcg<Ix2> {
    pub(super) fn push(&mut self, relevance: &[f32], k: &[usize]) -> Result<(), Panic> {
        self.0
            .push_column(Self::compute(relevance, k).view())
            .map_err(Into::into)
    }
}
