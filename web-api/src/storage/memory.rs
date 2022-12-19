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

// Assumptions:
// - there are only positive interactions (ie as in the current web engine) to avoid to store
//   redundant information
// - document ingestion and deletion is preferably done in a single batch to avoid to rebuild the
//   index for the embeddings too frequently

use std::{
    collections::{HashMap, HashSet},
    fmt,
    io::{Read, Write},
    mem,
};

use async_trait::async_trait;
use bincode::{deserialize_from, serialize_into, serialized_size};
use chrono::{DateTime, Local, NaiveDateTime};
use derive_more::Deref;
use instant_distance::{Builder as HnswBuilder, HnswMap, Point, Search};
use ouroboros::self_referencing;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use xayn_ai_coi::{cosine_similarity, Embedding, PositiveCoi, UserInterests};

use crate::{
    error::{
        application::Error,
        common::{DocumentNotFound, DocumentPropertyNotFound},
    },
    models::{
        DocumentId,
        DocumentProperties,
        DocumentProperty,
        DocumentPropertyId,
        IngestedDocument,
        PersonalizedDocument,
        UserId,
    },
    storage::{self, DeletionError, InsertionError, KnnSearchParams},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Document {
    properties: DocumentProperties,
    tags: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deref)]
struct EmbeddingRef<'a>(&'a Embedding);

impl Point for EmbeddingRef<'_> {
    fn distance(&self, other: &Self) -> f32 {
        1. - cosine_similarity(self.view(), other.view())
    }
}

#[self_referencing]
struct Embeddings {
    map: HashMap<DocumentId, Embedding>,
    #[borrows(map)]
    #[covariant]
    index: HnswMap<EmbeddingRef<'this>, &'this DocumentId>,
}

impl Embeddings {
    fn build(map: HashMap<DocumentId, Embedding>) -> Self {
        EmbeddingsBuilder {
            map,
            index_builder: |map| {
                let (embeddings, ids) = map
                    .iter()
                    .map(|(id, embedding)| (EmbeddingRef(embedding), id))
                    .unzip();
                HnswBuilder::default().build(embeddings, ids)
            },
        }
        .build()
    }
}

impl fmt::Debug for Embeddings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Embeddings")
            .field("map", self.borrow_map())
            .field("index", {
                let ids = &self.borrow_index().values;
                &self
                    .borrow_index()
                    .iter()
                    .filter_map(|(i, embedding)| {
                        ids.get(i.into_inner() as usize).map(|id| (id, embedding))
                    })
                    .collect::<HashMap<_, _>>()
            })
            .finish()
    }
}

impl Default for Embeddings {
    fn default() -> Self {
        Self::build(HashMap::default())
    }
}

#[derive(Debug, Default)]
pub(crate) struct Storage {
    documents: RwLock<(HashMap<DocumentId, Document>, Embeddings)>,
    interests: RwLock<HashMap<UserId, UserInterests>>,
    interactions: RwLock<HashMap<UserId, HashSet<(DocumentId, NaiveDateTime)>>>,
    users: RwLock<HashMap<UserId, NaiveDateTime>>,
    tags: RwLock<HashMap<UserId, HashMap<String, usize>>>,
}

#[async_trait]
impl storage::Document for Storage {
    async fn get_by_ids(&self, ids: &[&DocumentId]) -> Result<Vec<PersonalizedDocument>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let documents = self.documents.read().await;
        let documents = ids
            .iter()
            .filter_map(|&id| {
                documents.0.get(id).and_then(|document| {
                    documents
                        .1
                        .borrow_map()
                        .get(id)
                        .map(|embedding| PersonalizedDocument {
                            id: id.clone(),
                            score: 1.,
                            embedding: embedding.clone(),
                            properties: document.properties.clone(),
                            tags: document.tags.clone(),
                        })
                })
            })
            .collect();

        Ok(documents)
    }

    async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<'a>,
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        let excluded = params.excluded.iter().collect::<HashSet<_>>();
        let documents = self.documents.read().await;
        let documents = documents
            .1
            .borrow_index()
            .search(&EmbeddingRef(params.embedding), &mut Search::default())
            .filter_map(|item| {
                let id = *item.value;
                if excluded.contains(id) {
                    None
                } else {
                    documents.0.get(id).map(|document| PersonalizedDocument {
                        id: id.clone(),
                        score: item.distance,
                        embedding: item.point.0.clone(),
                        properties: document.properties.clone(),
                        tags: document.tags.clone(),
                    })
                }
            })
            .take(params.k_neighbors)
            .collect();

        Ok(documents)
    }

    async fn insert(
        &self,
        documents_embeddings: Vec<(IngestedDocument, Embedding)>,
    ) -> Result<(), InsertionError> {
        if documents_embeddings.is_empty() {
            return Ok(());
        }

        let mut documents = self.documents.write().await;
        let mut embeddings = mem::take(&mut documents.1).into_heads().map;
        documents.0.reserve(documents_embeddings.len());
        for (document, embedding) in documents_embeddings {
            documents.0.insert(
                document.id.clone(),
                Document {
                    properties: document.properties,
                    tags: document.tags,
                },
            );
            embeddings.insert(document.id, embedding);
        }
        documents.1 = Embeddings::build(embeddings);

        Ok(())
    }

    async fn delete(&self, ids: &[DocumentId]) -> Result<(), DeletionError> {
        if ids.is_empty() {
            return Ok(());
        }

        let mut documents = self.documents.write().await;
        let mut interactions = self.interactions.write().await;

        let ids = ids.iter().collect::<HashSet<_>>();
        documents.0.retain(|id, _| !ids.contains(id));
        let mut embeddings = mem::take(&mut documents.1).into_heads().map;
        embeddings.retain(|id, _| !ids.contains(id));
        documents.1 = Embeddings::build(embeddings);
        interactions.retain(|_, interactions| {
            interactions.retain(|(id, _)| !ids.contains(id));
            !interactions.is_empty()
        });

        Ok(())
    }
}

#[async_trait]
impl storage::DocumentProperties for Storage {
    async fn get(&self, id: &DocumentId) -> Result<Option<DocumentProperties>, Error> {
        let properties = self
            .documents
            .read()
            .await
            .0
            .get(id)
            .ok_or(DocumentNotFound)?
            .properties
            .clone();

        Ok(Some(properties))
    }

    async fn put(
        &self,
        id: &DocumentId,
        properties: &DocumentProperties,
    ) -> Result<Option<()>, Error> {
        self.documents
            .write()
            .await
            .0
            .get_mut(id)
            .ok_or(DocumentNotFound)?
            .properties = properties.clone();

        Ok(Some(()))
    }

    async fn delete(&self, id: &DocumentId) -> Result<Option<()>, Error> {
        self.documents
            .write()
            .await
            .0
            .get_mut(id)
            .ok_or(DocumentNotFound)?
            .properties
            .clear();

        Ok(Some(()))
    }
}

#[async_trait]
impl storage::DocumentProperty for Storage {
    async fn get(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
    ) -> Result<Option<Option<DocumentProperty>>, Error> {
        let property = self
            .documents
            .read()
            .await
            .0
            .get(document_id)
            .ok_or(DocumentNotFound)?
            .properties
            .get(property_id)
            .ok_or(DocumentPropertyNotFound)?
            .clone();

        Ok(Some(Some(property)))
    }

    async fn put(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
        property: &DocumentProperty,
    ) -> Result<Option<()>, Error> {
        self.documents
            .write()
            .await
            .0
            .get_mut(document_id)
            .ok_or(DocumentNotFound)?
            .properties
            .insert(property_id.clone(), property.clone());

        Ok(Some(()))
    }

    async fn delete(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
    ) -> Result<Option<()>, Error> {
        let property = self
            .documents
            .write()
            .await
            .0
            .get_mut(document_id)
            .ok_or(DocumentNotFound)?
            .properties
            .remove(property_id);

        if property.is_some() {
            Ok(Some(()))
        } else {
            Err(DocumentPropertyNotFound.into())
        }
    }
}

#[async_trait]
impl storage::Interest for Storage {
    async fn get(&self, id: &UserId) -> Result<UserInterests, Error> {
        let interests = self
            .interests
            .read()
            .await
            .get(id)
            .cloned()
            .unwrap_or_default();

        Ok(interests)
    }

    async fn update_positive<F>(
        &self,
        document_id: &DocumentId,
        user_id: &UserId,
        update_cois: F,
    ) -> Result<(), Error>
    where
        F: Fn(&mut Vec<PositiveCoi>) -> &PositiveCoi + Send + Sync,
    {
        let mut interests = self.interests.write().await;
        let mut interactions = self.interactions.write().await;

        let updated_coi = update_cois(&mut interests.entry(user_id.clone()).or_default().positive);
        let timestamp = DateTime::<Local>::from(updated_coi.stats.last_view).naive_local();
        interactions
            .entry(user_id.clone())
            .and_modify(|interactions| {
                interactions.insert((document_id.clone(), timestamp));
            })
            .or_insert_with(|| [(document_id.clone(), timestamp)].into());

        Ok(())
    }
}

#[async_trait]
impl storage::Interaction for Storage {
    async fn get(&self, id: &UserId) -> Result<Vec<DocumentId>, Error> {
        let document_ids = self
            .interactions
            .read()
            .await
            .get(id)
            .map(|interactions| {
                interactions
                    .iter()
                    .map(|(document_id, _)| document_id.clone())
                    .collect()
            })
            .unwrap_or_default();

        Ok(document_ids)
    }

    async fn user_seen(&self, id: &UserId) -> Result<(), Error> {
        self.users
            .write()
            .await
            .insert(id.clone(), Local::now().naive_local());

        Ok(())
    }
}

#[async_trait]
impl storage::Tag for Storage {
    async fn get(&self, id: &UserId) -> Result<HashMap<String, usize>, Error> {
        Ok(self.tags.read().await.get(id).cloned().unwrap_or_default())
    }

    async fn update(&self, id: &UserId, tags: &[String]) -> Result<(), Error> {
        if tags.is_empty() {
            return Ok(());
        }

        let mut tags_by_users = self.tags.write().await;
        if let Some(user_tags) = tags_by_users.get_mut(id) {
            for tag in tags {
                if let Some(weight) = user_tags.get_mut(tag) {
                    *weight += 1;
                } else {
                    user_tags.insert(tag.to_string(), 1);
                }
            }
        } else {
            tags_by_users.insert(
                id.clone(),
                tags.iter().map(|tag| (tag.to_string(), 1)).collect(),
            );
        }

        Ok(())
    }
}

#[allow(dead_code)]
impl Storage {
    pub(crate) async fn serialized_size(&self) -> Result<usize, bincode::Error> {
        let documents = self.documents.read().await;
        let interests = self.interests.read().await;
        let interactions = self.interactions.read().await;
        let users = self.users.read().await;
        let tags = self.tags.read().await;

        serialized_size(&(
            &documents.0,
            documents.1.borrow_map(),
            &*interests,
            &*interactions,
            &*users,
            &*tags,
        ))
        .map(
            #[allow(clippy::cast_possible_truncation)] // bounded by system architecture
            |size| size as usize,
        )
    }

    pub(crate) async fn serialize(&self, writer: impl Write) -> Result<(), bincode::Error> {
        let documents = self.documents.read().await;
        let interests = self.interests.read().await;
        let interactions = self.interactions.read().await;
        let users = self.users.read().await;
        let tags = self.tags.read().await;

        serialize_into(
            writer,
            &(
                &documents.0,
                documents.1.borrow_map(),
                &*interests,
                &*interactions,
                &*users,
                &*tags,
            ),
        )
    }

    pub(crate) fn deserialize(reader: impl Read) -> Result<Self, bincode::Error> {
        deserialize_from::<_, (_, HashMap<_, _>, _, _, _, _)>(reader).map(
            |(documents, embeddings, interests, interactions, users, tags)| Self {
                documents: RwLock::new((documents, Embeddings::build(embeddings))),
                interests: RwLock::new(interests),
                interactions: RwLock::new(interactions),
                users: RwLock::new(users),
                tags: RwLock::new(tags),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;

    #[tokio::test]
    async fn test_knn_search() {
        let ids = (0..3)
            .map(|id| DocumentId::new(id.to_string()).unwrap())
            .collect_vec();
        let documents = ids
            .iter()
            .map(|id| IngestedDocument {
                id: id.clone(),
                snippet: String::new(),
                properties: DocumentProperties::default(),
                tags: Vec::new(),
            })
            .collect_vec();
        let embeddings = [
            [1., 0., 0.].into(),
            [1., 1., 0.].into(),
            [1., 1., 1.].into(),
        ];
        let storage = Storage::default();
        storage::Document::insert(
            &storage,
            documents.iter().cloned().zip(embeddings).collect_vec(),
        )
        .await
        .unwrap();

        let embedding = &[0., 1., 1.].into();
        let documents = storage::Document::get_by_embedding(
            &storage,
            KnnSearchParams {
                excluded: &[],
                embedding,
                k_neighbors: 2,
                num_candidates: 2,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            documents.iter().map(|document| &document.id).collect_vec(),
            [&ids[2], &ids[1]],
        );

        let documents = storage::Document::get_by_embedding(
            &storage,
            KnnSearchParams {
                excluded: &[ids[1].clone()],
                embedding,
                k_neighbors: 3,
                num_candidates: 3,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            documents.iter().map(|document| &document.id).collect_vec(),
            [&ids[2], &ids[0]],
        );
    }
}
