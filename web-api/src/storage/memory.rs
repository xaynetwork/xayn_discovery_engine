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
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt,
    mem,
};

use async_trait::async_trait;
use bincode::{deserialize, serialize};
use chrono::{Local, NaiveDateTime, Utc, DateTime};
use derive_more::{AsRef, Deref};
use instant_distance::{Builder as HnswBuilder, HnswMap, Point, Search};
use ouroboros::self_referencing;
use serde::{de, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use tokio::sync::RwLock;
use xayn_ai_coi::{cosine_similarity, Embedding, PositiveCoi, UserInterests};

use super::{Document as _, InteractionUpdateContext};
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

#[derive(AsRef, Clone, Debug, Deref, Deserialize, Serialize)]
#[as_ref(forward)]
#[deref(forward)]
struct CowEmbedding<'a>(Cow<'a, Embedding>);

impl Point for CowEmbedding<'_> {
    fn distance(&self, other: &Self) -> f32 {
        1. - cosine_similarity(self.view(), other.view())
    }
}

#[self_referencing]
struct Embeddings {
    map: HashMap<DocumentId, Embedding>,
    #[borrows(map)]
    #[covariant]
    index: HnswMap<CowEmbedding<'this>, Cow<'this, DocumentId>>,
}

impl Embeddings {
    fn borrowed(map: HashMap<DocumentId, Embedding>) -> Self {
        EmbeddingsBuilder {
            map,
            index_builder: |map| {
                let (embeddings, ids) = map
                    .iter()
                    .map(|(id, embedding)| {
                        (CowEmbedding(Cow::Borrowed(embedding)), Cow::Borrowed(id))
                    })
                    .unzip();
                HnswBuilder::default().build(embeddings, ids)
            },
        }
        .build()
    }

    fn owned(
        map: HashMap<DocumentId, Embedding>,
        index: HnswMap<CowEmbedding<'static>, Cow<'static, DocumentId>>,
    ) -> Self {
        EmbeddingsBuilder {
            map,
            index_builder: |_| index,
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
        Self::borrowed(HashMap::default())
    }
}

impl Serialize for Embeddings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Embeddings", 2)?;
        state.serialize_field("map", self.borrow_map())?;
        state.serialize_field("index", self.borrow_index())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Embeddings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Map,
            Index,
        }

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Embeddings;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("struct Embeddings")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let map = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let index = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;

                Ok(Embeddings::owned(map, index))
            }

            fn visit_map<A>(self, mut de_map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut map = None;
                let mut index = None;
                while let Some(key) = de_map.next_key()? {
                    match key {
                        Field::Map => {
                            if map.is_some() {
                                return Err(de::Error::duplicate_field("map"));
                            }
                            map = Some(de_map.next_value()?);
                        }
                        Field::Index => {
                            if index.is_some() {
                                return Err(de::Error::duplicate_field("index"));
                            }
                            index = Some(de_map.next_value()?);
                        }
                    }
                }

                Ok(Embeddings::owned(
                    map.ok_or_else(|| de::Error::missing_field("map"))?,
                    index.ok_or_else(|| de::Error::missing_field("index"))?,
                ))
            }
        }

        deserializer.deserialize_struct("Embeddings", &["map", "index"], Visitor)
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
            .search(
                &CowEmbedding(Cow::Borrowed(params.embedding)),
                &mut Search::default(),
            )
            .filter_map(|item| {
                let id = item.value.as_ref();
                if excluded.contains(id) {
                    None
                } else {
                    documents.0.get(id).map(|document| PersonalizedDocument {
                        id: id.clone(),
                        score: item.distance,
                        embedding: item.point.as_ref().clone(),
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
        documents.1 = Embeddings::borrowed(embeddings);

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
        documents.1 = Embeddings::borrowed(embeddings);
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

    async fn update_interactions<F>(
        &self,
        user_id: &UserId,
        updated_document_ids: &[&DocumentId],
        mut update_logic: F,
    ) -> Result<(), Error>
    where
        F: for<'a, 'b> FnMut(InteractionUpdateContext<'a, 'b>) -> PositiveCoi + Send + Sync,
    {
        // Note: This doesn't have the exact same concurrency semantics as the postgres version
        let documents = self.get_by_ids(updated_document_ids).await?;
        let mut interests = self.interests.write().await;
        let mut interactions = self.interactions.write().await;
        let interactions = interactions.entry(user_id.clone()).or_default();
        let mut tags = self.tags.write().await;
        let tags = tags.entry(user_id.clone()).or_default();

        let positive_cois = &mut interests.entry(user_id.clone()).or_default().positive;

        let mut tag_weight_diff = documents
            .iter()
            .flat_map(|d| &d.tags)
            .map(|tag| (tag.as_str(), 0))
            .collect::<HashMap<_, _>>();

        for document in &documents {
            let updated = update_logic(InteractionUpdateContext {
                document,
                tag_weight_diff: &mut tag_weight_diff,
                positive_cois,
            });
            interactions.insert((
                document.id.clone(),
                DateTime::<Utc>::from(updated.stats.last_view).naive_utc(),
            ));
        }

        for (&tag, &diff) in &tag_weight_diff {
            if let Some(weight) = tags.get_mut(tag) {
                //FIXME use `saturating_add_signed` when stabilized
                let abs_diff = diff.unsigned_abs() as usize;
                if diff < 0 {
                    *weight -= abs_diff;
                } else {
                    *weight += abs_diff;
                }
            } else {
                tags.insert(tag.to_owned(), diff.try_into().unwrap_or_default());
            }
        }

        Ok(())
    }
}

#[async_trait]
impl storage::Tag for Storage {
    async fn get(&self, id: &UserId) -> Result<HashMap<String, usize>, Error> {
        Ok(self.tags.read().await.get(id).cloned().unwrap_or_default())
    }
}

impl Storage {
    pub(crate) async fn serialize(&self) -> Result<Vec<u8>, bincode::Error> {
        serialize(&(
            &*self.documents.read().await,
            &*self.interests.read().await,
            &*self.interactions.read().await,
            &*self.users.read().await,
            &*self.tags.read().await,
        ))
    }

    pub(crate) fn deserialize(bytes: &[u8]) -> Result<Self, bincode::Error> {
        deserialize(bytes).map(|(documents, interests, interactions, users, tags)| Self {
            documents: RwLock::new(documents),
            interests: RwLock::new(interests),
            interactions: RwLock::new(interactions),
            users: RwLock::new(users),
            tags: RwLock::new(tags),
        })
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

    #[tokio::test]
    async fn test_serde() {
        let storage = Storage::default();
        storage::Document::insert(
            &storage,
            vec![(
                IngestedDocument {
                    id: DocumentId::new("42").unwrap(),
                    snippet: "snippet".into(),
                    properties: DocumentProperties::default(),
                    tags: vec!["tag".into()],
                },
                [1., 2., 3.].into(),
            )],
        )
        .await
        .unwrap();
        //TODO[pmk]
        // storage::Interaction::update_interactions (&storage, &UserId::new("abc").unwrap(), &["tag".into()])
        //     .await
        //     .unwrap();

        let storage = Storage::deserialize(&storage.serialize().await.unwrap()).unwrap();
        let documents = storage::Document::get_by_ids(&storage, &[&DocumentId::new("42").unwrap()])
            .await
            .unwrap();
        assert_eq!(documents[0].id, DocumentId::new("42").unwrap());
        assert_eq!(documents[0].embedding, Embedding::from([1., 2., 3.]));
        assert!(documents[0].properties.is_empty());
        // assert_eq!(documents[0].tags, vec![String::from("tag")]);
        // assert_eq!(
        //     storage::Tag::get(&storage, &UserId::new("abc").unwrap())
        //         .await
        //         .unwrap(),
        //     HashMap::from([(String::from("tag"), 1)]),
        // );
    }
}
