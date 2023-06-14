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
use chrono::{DateTime, Utc};
use derive_more::{AsRef, Deref};
use instant_distance::{Builder as HnswBuilder, HnswMap, Point, Search};
use ouroboros::self_referencing;
use serde::{de, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use tokio::sync::RwLock;
use xayn_ai_bert::NormalizedEmbedding;
use xayn_ai_coi::Coi;

use super::{Document as _, InteractionUpdateContext, TagWeights};
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
        DocumentSnippet,
        DocumentTag,
        ExcerptedDocument,
        IngestedDocument,
        InteractedDocument,
        PersonalizedDocument,
        UserId,
    },
    storage::{self, KnnSearchParams, Warning},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Document {
    snippet: DocumentSnippet,
    properties: DocumentProperties,
    tags: Vec<DocumentTag>,
    is_candidate: bool,
}

#[derive(AsRef, Clone, Debug, Deref, Deserialize, Serialize)]
#[as_ref(forward)]
#[deref(forward)]
struct CowEmbedding<'a>(Cow<'a, NormalizedEmbedding>);

impl Point for CowEmbedding<'_> {
    fn distance(&self, other: &Self) -> f32 {
        1. - self.dot_product(other)
    }
}

#[self_referencing]
struct Embeddings {
    map: HashMap<DocumentId, NormalizedEmbedding>,
    #[borrows(map)]
    #[covariant]
    index: HnswMap<CowEmbedding<'this>, Cow<'this, DocumentId>>,
}

impl Embeddings {
    fn borrowed(map: HashMap<DocumentId, NormalizedEmbedding>) -> Self {
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
        map: HashMap<DocumentId, NormalizedEmbedding>,
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
    interests: RwLock<HashMap<UserId, Vec<Coi>>>,
    #[allow(clippy::type_complexity)]
    interactions: RwLock<HashMap<UserId, HashSet<(DocumentId, DateTime<Utc>)>>>,
    users: RwLock<HashMap<UserId, DateTime<Utc>>>,
    tags: RwLock<HashMap<UserId, HashMap<DocumentTag, usize>>>,
}

#[async_trait(?Send)]
impl storage::Document for Storage {
    async fn get_interacted(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Vec<InteractedDocument>, Error> {
        let documents = self.documents.read().await;
        let documents = ids
            .into_iter()
            .filter_map(|id| {
                documents.0.get(id).and_then(|document| {
                    documents
                        .1
                        .borrow_map()
                        .get(id)
                        .map(|embedding| InteractedDocument {
                            id: id.clone(),
                            embedding: embedding.clone(),
                            tags: document.tags.clone(),
                        })
                })
            })
            .collect();

        Ok(documents)
    }

    async fn get_personalized(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        let documents = self.documents.read().await;
        let documents = ids
            .into_iter()
            .filter_map(|id| {
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

    async fn get_excerpted(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Vec<ExcerptedDocument>, Error> {
        let documents = self.documents.read().await;
        let documents = ids
            .into_iter()
            .filter_map(|id| {
                documents.0.get(id).map(|document| ExcerptedDocument {
                    id: id.clone(),
                    snippet: document.snippet.clone(),
                    properties: document.properties.clone(),
                    tags: document.tags.clone(),
                    is_candidate: document.is_candidate,
                })
            })
            .collect();

        Ok(documents)
    }

    async fn get_embedding(&self, id: &DocumentId) -> Result<Option<NormalizedEmbedding>, Error> {
        Ok(self.documents.read().await.1.borrow_map().get(id).cloned())
    }

    async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<
            'a,
            impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &'a DocumentId>>,
        >,
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        if params.published_after.is_some() {
            unimplemented!(/* we don't need it for memory.rs */);
        }

        let excluded = params.excluded.into_iter().collect::<HashSet<_>>();
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
            .take(params.count)
            .collect();

        Ok(documents)
    }

    async fn insert(
        &self,
        new_documents: Vec<IngestedDocument>,
    ) -> Result<Warning<DocumentId>, Error> {
        if new_documents.is_empty() {
            return Ok(Warning::default());
        }

        let mut documents = self.documents.write().await;
        let mut embeddings = mem::take(&mut documents.1).into_heads().map;
        documents.0.reserve(new_documents.len());
        for document in new_documents {
            documents.0.insert(
                document.id.clone(),
                Document {
                    snippet: document.snippet,
                    properties: document.properties,
                    tags: document.tags,
                    is_candidate: document.is_candidate,
                },
            );
            embeddings.insert(document.id, document.embedding);
        }
        documents.1 = Embeddings::borrowed(embeddings);

        Ok(Warning::default())
    }

    async fn delete(
        &self,
        ids: impl IntoIterator<IntoIter = impl Clone + ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Warning<DocumentId>, Error> {
        let mut documents = self.documents.write().await;
        let mut interactions = self.interactions.write().await;

        let mut ids = ids.into_iter().collect::<HashSet<_>>();
        interactions.retain(|_, interactions| {
            interactions.retain(|(id, _)| !ids.contains(id));
            !interactions.is_empty()
        });
        documents.0.retain(|id, _| !ids.contains(id));
        let mut embeddings = mem::take(&mut documents.1).into_heads().map;
        embeddings.retain(|id, _| !ids.remove(id));
        documents.1 = Embeddings::borrowed(embeddings);

        Ok(ids.into_iter().cloned().collect())
    }
}

#[async_trait(?Send)]
impl storage::DocumentCandidate for Storage {
    async fn get(&self) -> Result<Vec<DocumentId>, Error> {
        let documents = self
            .documents
            .read()
            .await
            .0
            .iter()
            .filter_map(|(id, document)| document.is_candidate.then(|| id.clone()))
            .collect();

        Ok(documents)
    }

    async fn set(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<Warning<DocumentId>, Error> {
        let mut ids = ids.into_iter().collect::<HashSet<_>>();
        for (id, document) in &mut self.documents.write().await.0 {
            if ids.remove(id) ^ document.is_candidate {
                document.is_candidate = !document.is_candidate;
            }
        }

        Ok(ids.into_iter().cloned().collect())
    }

    async fn add(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<Warning<DocumentId>, Error> {
        let documents = &mut self.documents.write().await.0;
        let mut failed = Warning::default();
        for id in ids {
            if let Some(document) = documents.get_mut(id) {
                document.is_candidate = true;
            } else {
                failed.push(id.clone());
            }
        }

        Ok(failed)
    }

    async fn remove(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<Warning<DocumentId>, Error> {
        let documents = &mut self.documents.write().await.0;
        let mut failed = Warning::default();
        for id in ids {
            if let Some(document) = documents.get_mut(id) {
                document.is_candidate = false;
            } else {
                failed.push(id.clone());
            }
        }

        Ok(failed)
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
    ) -> Result<Option<Option<()>>, Error> {
        self.documents
            .write()
            .await
            .0
            .get_mut(document_id)
            .ok_or(DocumentNotFound)?
            .properties
            .remove(property_id)
            .ok_or(DocumentPropertyNotFound)?;

        Ok(Some(Some(())))
    }
}

#[async_trait]
impl storage::Interest for Storage {
    async fn get(&self, id: &UserId) -> Result<Vec<Coi>, Error> {
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

#[async_trait(?Send)]
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

    async fn user_seen(&self, id: &UserId, time: DateTime<Utc>) -> Result<(), Error> {
        self.users.write().await.insert(id.clone(), time);

        Ok(())
    }

    async fn update_interactions(
        &self,
        user_id: &UserId,
        interactions: impl IntoIterator<IntoIter = impl Clone + ExactSizeIterator<Item = &DocumentId>>,
        store_user_history: bool,
        time: DateTime<Utc>,
        mut update_logic: impl for<'a, 'b> FnMut(InteractionUpdateContext<'a, 'b>) -> Coi,
    ) -> Result<(), Error> {
        // Note: This doesn't has the exact same concurrency semantics as the postgres version
        let documents = self.get_interacted(interactions).await?;
        let mut interests = self.interests.write().await;
        let mut interactions = self.interactions.write().await;
        let interactions = interactions.entry(user_id.clone()).or_default();
        let mut tags = self.tags.write().await;
        let tags = tags.entry(user_id.clone()).or_default();

        let interests = interests.entry(user_id.clone()).or_default();

        let mut tag_weight_diff = documents
            .iter()
            .flat_map(|document| &document.tags)
            .map(|tag| (tag, 0))
            .collect::<HashMap<_, _>>();

        for document in &documents {
            let updated = update_logic(InteractionUpdateContext {
                document,
                tag_weight_diff: &mut tag_weight_diff,
                interests,
                time,
            });
            if store_user_history {
                interactions.insert((document.id.clone(), updated.stats.last_view));
            }
        }

        for (tag, diff) in tag_weight_diff {
            if let Some(weight) = tags.get_mut(tag) {
                *weight = weight.saturating_add_signed(diff as isize);
            } else {
                tags.insert(tag.clone(), diff.try_into().unwrap_or_default());
            }
        }

        Ok(())
    }
}

#[async_trait]
impl storage::Tag for Storage {
    async fn get(&self, id: &UserId) -> Result<TagWeights, Error> {
        Ok(self.tags.read().await.get(id).cloned().unwrap_or_default())
    }

    async fn put(
        &self,
        document_id: &DocumentId,
        tags: &[DocumentTag],
    ) -> Result<Option<()>, Error> {
        if let Some(document) = self.documents.write().await.0.get_mut(document_id) {
            document.tags = tags.to_vec();
            Ok(Some(()))
        } else {
            Ok(None)
        }
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
    use xayn_ai_coi::CoiId;
    use xayn_test_utils::assert_approx_eq;

    use super::*;

    #[tokio::test]
    async fn test_knn_search() {
        let ids = (0..3)
            .map(|id| DocumentId::try_from(id.to_string()).unwrap())
            .collect_vec();
        let embeddings = [
            [1., 0., 0.].try_into().unwrap(),
            [1., 1., 0.].try_into().unwrap(),
            [1., 1., 1.].try_into().unwrap(),
        ];
        let documents = ids
            .iter()
            .zip(embeddings)
            .map(|(id, embedding)| IngestedDocument {
                id: id.clone(),
                snippet: "snippet".try_into().unwrap(),
                properties: DocumentProperties::default(),
                tags: Vec::new(),
                embedding,
                is_candidate: true,
            })
            .collect_vec();
        let storage = Storage::default();
        storage::Document::insert(&storage, documents)
            .await
            .unwrap();

        let embedding = &[0., 1., 1.].try_into().unwrap();
        let documents = storage::Document::get_by_embedding(
            &storage,
            KnnSearchParams {
                excluded: [],
                embedding,
                count: 2,
                num_candidates: 2,
                published_after: None,
                min_similarity: None,
                query: None,
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
                excluded: [&ids[1]],
                embedding,
                count: 3,
                num_candidates: 3,
                published_after: None,
                min_similarity: None,
                query: None,
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
        let doc_id = DocumentId::try_from("42").unwrap();
        let snippet = DocumentSnippet::try_from("snippet").unwrap();
        let tags = vec![DocumentTag::try_from("tag").unwrap()];
        let embedding = NormalizedEmbedding::try_from([1., 2., 3.]).unwrap();
        storage::Document::insert(
            &storage,
            vec![IngestedDocument {
                id: doc_id.clone(),
                snippet: snippet.clone(),
                properties: DocumentProperties::default(),
                tags: tags.clone(),
                embedding: embedding.clone(),
                is_candidate: true,
            }],
        )
        .await
        .unwrap();
        let user_id = UserId::try_from("abc").unwrap();
        storage::Interaction::update_interactions(
            &storage,
            &user_id,
            [&doc_id],
            true,
            Utc::now(),
            |context| {
                *context.tag_weight_diff.get_mut(&tags[0]).unwrap() += 10;
                let coi = Coi::new(
                    CoiId::new(),
                    [0.2, 9.4, 1.2].try_into().unwrap(),
                    context.time,
                );
                context.interests.push(coi.clone());
                coi
            },
        )
        .await
        .unwrap();

        let storage = Storage::deserialize(&storage.serialize().await.unwrap()).unwrap();
        let documents = storage::Document::get_excerpted(&storage, [&doc_id])
            .await
            .unwrap();
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].id, doc_id);
        assert_eq!(documents[0].snippet, snippet);
        let documents = storage::Document::get_personalized(&storage, [&doc_id])
            .await
            .unwrap();
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].id, doc_id);
        assert_approx_eq!(f32, documents[0].embedding, embedding);
        assert!(documents[0].properties.is_empty());
        assert_eq!(documents[0].tags, tags);
        assert_eq!(
            storage::Tag::get(&storage, &user_id).await.unwrap(),
            HashMap::from([(tags[0].clone(), 10)]),
        );
    }
}
