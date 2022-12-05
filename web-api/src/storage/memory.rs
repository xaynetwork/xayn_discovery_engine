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
// - document and user ids follow the mind format (ie they are unique u32s prefixed with a "N" resp
//   "U") to allow for faster map access without hashing (ie each u32 is already its own hash)
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
use derive_more::{Deref, Display};
use fnv::{FnvHashMap, FnvHashSet};
use instant_distance::{Builder as HnswBuilder, HnswMap, Point, Search};
use itertools::Itertools;
use ouroboros::self_referencing;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use xayn_ai_coi::{cosine_similarity, Embedding, PositiveCoi, UserInterests};

use crate::{
    error::{
        application::Error,
        common::{
            DocumentIdAsObject,
            DocumentNotFound,
            DocumentPropertyNotFound,
            InvalidDocumentId,
            InvalidUserId,
        },
    },
    models::{
        self,
        DocumentProperties,
        DocumentProperty,
        DocumentPropertyId,
        IngestedDocument,
        PersonalizedDocument,
    },
    storage::{self, InsertionError, KnnSearchParams},
};

#[derive(Clone, Copy, Debug, Deserialize, Display, Eq, Hash, PartialEq, Serialize)]
#[display(fmt = "N{_0}")]
struct DocumentId(u32);

impl TryFrom<&models::DocumentId> for DocumentId {
    type Error = InvalidDocumentId;

    fn try_from(id: &models::DocumentId) -> Result<Self, Self::Error> {
        id.as_ref()
            .trim()
            .trim_start_matches('N')
            .parse()
            .map(Self)
            .map_err(|_| InvalidDocumentId { id: id.to_string() })
    }
}

impl TryFrom<&DocumentId> for models::DocumentId {
    type Error = InvalidDocumentId;

    fn try_from(id: &DocumentId) -> Result<Self, Self::Error> {
        Self::new(id.to_string())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Document {
    properties: DocumentProperties,
    category: Option<String>,
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
    map: FnvHashMap<DocumentId, Embedding>,
    #[borrows(map)]
    #[covariant]
    index: HnswMap<EmbeddingRef<'this>, &'this DocumentId>,
}

impl Embeddings {
    fn build(map: FnvHashMap<DocumentId, Embedding>) -> Self {
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
                    .collect::<FnvHashMap<_, _>>()
            })
            .finish()
    }
}

impl Default for Embeddings {
    fn default() -> Self {
        Self::build(FnvHashMap::default())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Display, Eq, Hash, PartialEq, Serialize)]
#[display(fmt = "U{_0}")]
struct UserId(u32);

impl TryFrom<&models::UserId> for UserId {
    type Error = InvalidUserId;

    fn try_from(id: &models::UserId) -> Result<Self, Self::Error> {
        id.as_ref()
            .trim()
            .trim_start_matches('U')
            .parse()
            .map(Self)
            .map_err(|_| InvalidUserId { id: id.to_string() })
    }
}

#[derive(Debug, Default)]
pub(crate) struct Storage {
    documents: RwLock<(FnvHashMap<DocumentId, Document>, Embeddings)>,
    interests: RwLock<FnvHashMap<UserId, UserInterests>>,
    interactions: RwLock<FnvHashMap<UserId, HashSet<(DocumentId, NaiveDateTime)>>>,
    users: RwLock<FnvHashMap<UserId, NaiveDateTime>>,
    categories: RwLock<FnvHashMap<UserId, HashMap<String, usize>>>,
}

#[async_trait]
impl storage::Document for Storage {
    async fn get_by_ids(
        &self,
        ids: &[&models::DocumentId],
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let documents = self.documents.read().await;
        ids.iter()
            .filter_map(|&id| {
                id.try_into()
                    .map(|document_id| {
                        documents.0.get(&document_id).and_then(|document| {
                            documents.1.borrow_map().get(&document_id).map(|embedding| {
                                PersonalizedDocument {
                                    id: id.clone(),
                                    score: 1.,
                                    embedding: embedding.clone(),
                                    properties: document.properties.clone(),
                                    category: document.category.clone(),
                                }
                            })
                        })
                    })
                    .transpose()
            })
            .try_collect()
            .map_err(Into::into)
    }

    async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<'a>,
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        let excluded = params
            .excluded
            .iter()
            .map(TryInto::try_into)
            .try_collect::<_, FnvHashSet<DocumentId>, _>()?;
        let documents = self.documents.read().await;
        documents
            .1
            .borrow_index()
            .search(&EmbeddingRef(params.embedding), &mut Search::default())
            .filter_map(|item| {
                let id = *item.value;
                if excluded.contains(id) {
                    None
                } else {
                    documents.0.get(id).map(|document| {
                        id.try_into().map(|id| PersonalizedDocument {
                            id,
                            score: item.distance,
                            embedding: item.point.0.clone(),
                            properties: document.properties.clone(),
                            category: document.category.clone(),
                        })
                    })
                }
            })
            .take(params.k_neighbors)
            .try_collect()
            .map_err(Into::into)
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
        let failed_documents = documents_embeddings
            .into_iter()
            .filter_map(|(document, embedding)| {
                (&document.id).try_into().map_or_else(
                    |_| Some(DocumentIdAsObject { id: document.id }),
                    |document_id| {
                        documents.0.insert(
                            document_id,
                            Document {
                                properties: document.properties,
                                category: document.category,
                            },
                        );
                        embeddings.insert(document_id, embedding);
                        None
                    },
                )
            })
            .collect_vec();
        documents.1 = Embeddings::build(embeddings);

        if failed_documents.is_empty() {
            Ok(())
        } else {
            Err(InsertionError::PartialFailure { failed_documents })
        }
    }

    async fn delete(&self, ids: &[models::DocumentId]) -> Result<(), Error> {
        if ids.is_empty() {
            return Ok(());
        }

        let ids = ids
            .iter()
            .map(TryInto::try_into)
            .try_collect::<_, FnvHashSet<DocumentId>, _>()?;
        let mut documents = self.documents.write().await;
        documents.0.retain(|id, _| !ids.contains(id));
        let mut embeddings = mem::take(&mut documents.1).into_heads().map;
        embeddings.retain(|id, _| !ids.contains(id));
        documents.1 = Embeddings::build(embeddings);

        Ok(())
    }
}

#[async_trait]
impl storage::DocumentProperties for Storage {
    async fn get(&self, id: &models::DocumentId) -> Result<Option<DocumentProperties>, Error> {
        let id = id.try_into()?;
        let properties = self
            .documents
            .read()
            .await
            .0
            .get(&id)
            .ok_or(DocumentNotFound)?
            .properties
            .clone();

        Ok(Some(properties))
    }

    async fn put(
        &self,
        id: &models::DocumentId,
        properties: &DocumentProperties,
    ) -> Result<Option<()>, Error> {
        let id = id.try_into()?;
        self.documents
            .write()
            .await
            .0
            .get_mut(&id)
            .ok_or(DocumentNotFound)?
            .properties = properties.clone();

        Ok(Some(()))
    }

    async fn delete(&self, id: &models::DocumentId) -> Result<Option<()>, Error> {
        let id = id.try_into()?;
        self.documents
            .write()
            .await
            .0
            .get_mut(&id)
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
        document_id: &models::DocumentId,
        property_id: &DocumentPropertyId,
    ) -> Result<Option<Option<DocumentProperty>>, Error> {
        let document_id = document_id.try_into()?;
        let property = self
            .documents
            .read()
            .await
            .0
            .get(&document_id)
            .ok_or(DocumentNotFound)?
            .properties
            .get(property_id)
            .ok_or(DocumentPropertyNotFound)?
            .clone();

        Ok(Some(Some(property)))
    }

    async fn put(
        &self,
        document_id: &models::DocumentId,
        property_id: &DocumentPropertyId,
        property: &DocumentProperty,
    ) -> Result<Option<()>, Error> {
        let document_id = document_id.try_into()?;
        self.documents
            .write()
            .await
            .0
            .get_mut(&document_id)
            .ok_or(DocumentNotFound)?
            .properties
            .insert(property_id.clone(), property.clone());

        Ok(Some(()))
    }

    async fn delete(
        &self,
        document_id: &models::DocumentId,
        property_id: &DocumentPropertyId,
    ) -> Result<Option<()>, Error> {
        let document_id = document_id.try_into()?;
        let property = self
            .documents
            .write()
            .await
            .0
            .get_mut(&document_id)
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
    async fn get(&self, user_id: &models::UserId) -> Result<UserInterests, Error> {
        let user_id = user_id.try_into()?;
        let interests = self
            .interests
            .read()
            .await
            .get(&user_id)
            .cloned()
            .unwrap_or_default();

        Ok(interests)
    }

    async fn update_positive<F>(
        &self,
        doc_id: &models::DocumentId,
        user_id: &models::UserId,
        update_cois: F,
    ) -> Result<(), Error>
    where
        F: Fn(&mut Vec<PositiveCoi>) -> &PositiveCoi + Send + Sync,
    {
        let document_id = doc_id.try_into()?;
        let user_id = user_id.try_into()?;
        let mut interests = self.interests.write().await;
        let mut interactions = self.interactions.write().await;

        let updated_coi = update_cois(&mut interests.entry(user_id).or_default().positive);
        let timestamp = DateTime::<Local>::from(updated_coi.stats.last_view).naive_local();
        interactions
            .entry(user_id)
            .and_modify(|interactions| {
                interactions.insert((document_id, timestamp));
            })
            .or_insert_with(|| [(document_id, timestamp)].into());

        Ok(())
    }
}

#[async_trait]
impl storage::Interaction for Storage {
    async fn get(&self, user_id: &models::UserId) -> Result<Vec<models::DocumentId>, Error> {
        let id = user_id.try_into()?;
        if let Some(interactions) = self.interactions.read().await.get(&id) {
            interactions
                .iter()
                .map(|(document_id, _)| document_id.try_into().map_err(Into::into))
                .try_collect()
        } else {
            Ok(Vec::new())
        }
    }

    async fn delete(&self, documents: &[models::DocumentId]) -> Result<(), Error> {
        if documents.is_empty() {
            return Ok(());
        }

        let ids = documents
            .iter()
            .map(TryInto::try_into)
            .try_collect::<_, FnvHashSet<DocumentId>, _>()?;
        self.interactions.write().await.retain(|_, interactions| {
            interactions.retain(|(id, _)| !ids.contains(id));
            !interactions.is_empty()
        });

        Ok(())
    }

    async fn user_seen(&self, id: &models::UserId) -> Result<(), Error> {
        let id = id.try_into()?;
        self.users
            .write()
            .await
            .insert(id, Local::now().naive_local());

        Ok(())
    }
}

#[async_trait]
impl storage::Category for Storage {
    async fn get(&self, user_id: &models::UserId) -> Result<HashMap<String, usize>, Error> {
        let id = user_id.try_into()?;
        let categories = self
            .categories
            .read()
            .await
            .get(&id)
            .cloned()
            .unwrap_or_default();

        Ok(categories)
    }

    async fn update(&self, user_id: &models::UserId, category: &str) -> Result<(), Error> {
        let id = user_id.try_into()?;
        self.categories
            .write()
            .await
            .entry(id)
            .and_modify(|categories| {
                categories
                    .entry(category.to_string())
                    .and_modify(|weight| *weight += 1)
                    .or_insert(1);
            })
            .or_insert_with(|| [(category.to_string(), 1)].into());

        Ok(())
    }
}

#[allow(dead_code)]
impl Storage {
    pub(crate) fn document(&self) -> &impl storage::Document {
        self
    }

    pub(crate) fn document_properties(&self) -> &impl storage::DocumentProperties {
        self
    }

    pub(crate) fn document_property(&self) -> &impl storage::DocumentProperty {
        self
    }

    pub(crate) fn interest(&self) -> &impl storage::Interest {
        self
    }

    pub(crate) fn interaction(&self) -> &impl storage::Interaction {
        self
    }

    pub(crate) fn category(&self) -> &impl storage::Category {
        self
    }

    pub(crate) async fn serialized_size(&self) -> Result<usize, bincode::Error> {
        let documents = self.documents.read().await;
        let interests = self.interests.read().await;
        let interactions = self.interactions.read().await;
        let users = self.users.read().await;
        let categories = self.categories.read().await;

        serialized_size(&(
            &documents.0,
            documents.1.borrow_map(),
            &*interests,
            &*interactions,
            &*users,
            &*categories,
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
        let categories = self.categories.read().await;

        serialize_into(
            writer,
            &(
                &documents.0,
                documents.1.borrow_map(),
                &*interests,
                &*interactions,
                &*users,
                &*categories,
            ),
        )
    }

    pub(crate) fn deserialize(reader: impl Read) -> Result<Self, bincode::Error> {
        deserialize_from::<_, (_, FnvHashMap<_, _>, _, _, _, _)>(reader).map(
            |(documents, embeddings, interests, interactions, users, categories)| Self {
                documents: RwLock::new((documents, Embeddings::build(embeddings))),
                interests: RwLock::new(interests),
                interactions: RwLock::new(interactions),
                users: RwLock::new(users),
                categories: RwLock::new(categories),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Document;

    #[tokio::test]
    async fn test_knn_search() {
        let ids = (0..3)
            .map(|id| models::DocumentId::new(format!("N{id}")).unwrap())
            .collect_vec();
        let documents = ids
            .iter()
            .map(|id| IngestedDocument {
                id: id.clone(),
                snippet: String::new(),
                properties: DocumentProperties::default(),
                category: None,
            })
            .collect_vec();
        let embeddings = [
            [1., 0., 0.].into(),
            [1., 1., 0.].into(),
            [1., 1., 1.].into(),
        ];
        let storage = Storage::default();
        storage
            .document()
            .insert(documents.iter().cloned().zip(embeddings).collect_vec())
            .await
            .unwrap();

        let embedding = &[0., 1., 1.].into();
        let documents = storage
            .document()
            .get_by_embedding(KnnSearchParams {
                excluded: &[],
                embedding,
                k_neighbors: 2,
                num_candidates: 2,
            })
            .await
            .unwrap();
        assert_eq!(
            documents.iter().map(|document| &document.id).collect_vec(),
            [&ids[2], &ids[1]],
        );

        let documents = storage
            .document()
            .get_by_embedding(KnnSearchParams {
                excluded: &[ids[1].clone()],
                embedding,
                k_neighbors: 3,
                num_candidates: 3,
            })
            .await
            .unwrap();
        assert_eq!(
            documents.iter().map(|document| &document.id).collect_vec(),
            [&ids[2], &ids[0]],
        );
    }
}
