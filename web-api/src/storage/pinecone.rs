// Copyright 2023 Xayn AG
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

mod client;

use std::collections::HashMap;

pub(crate) use client::{Client, ClientBuilder};
use itertools::Itertools;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use xayn_ai_bert::{NormalizedEmbedding, NormalizedSparseEmbedding};
use xayn_web_api_shared::url::{NO_BODY, NO_PARAMS};

use crate::{
    error::application::Error,
    models::{DocumentId, DocumentProperties, DocumentProperty, DocumentPropertyId},
    storage::{utils::IgnoredResponse, IngestedDocument, KnnSearchParams, Warning},
};

// TODO: remove after testing
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(super) struct Namespace {
    #[serde(rename = "vectorCount")]
    count: usize,
}

// TODO: remove after testing
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(super) struct IndexStats {
    namespaces: HashMap<String, Namespace>,
    dimension: usize,
    #[serde(rename = "indexFullness")]
    fullness: f32,
    #[serde(rename = "totalVectorCount")]
    count: usize,
}

type UpdatedDocument<'a> = (
    &'a DocumentId,
    Option<&'a NormalizedEmbedding>,
    Option<&'a NormalizedSparseEmbedding>,
    &'a DocumentProperties,
);

#[derive(Debug, Deserialize)]
struct FetchedDocument {
    id: DocumentId,
    #[serde(rename = "values")]
    embedding: NormalizedEmbedding,
    #[serde(rename = "sparseValues")]
    sparse_embedding: NormalizedSparseEmbedding,
    #[serde(
        default,
        deserialize_with = "DocumentProperties::deserialize_time_from_int",
        rename = "metadata"
    )]
    properties: DocumentProperties,
}

#[derive(Debug, Default, Deserialize)]
struct FetchedDocuments {
    #[serde(rename = "vectors")]
    documents: HashMap<DocumentId, FetchedDocument>,
}

impl IntoIterator for FetchedDocuments {
    type Item = <HashMap<DocumentId, FetchedDocument> as IntoIterator>::Item;
    type IntoIter = <HashMap<DocumentId, FetchedDocument> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.documents.into_iter()
    }
}

impl Extend<<FetchedDocuments as IntoIterator>::Item> for FetchedDocuments {
    fn extend<T>(&mut self, documents: T)
    where
        T: IntoIterator<Item = <FetchedDocuments as IntoIterator>::Item>,
    {
        self.documents.extend(documents);
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct QueriedDocument {
    id: DocumentId,
    score: f32,
}

#[derive(Debug, Deserialize)]
pub(super) struct QueriedDocuments {
    #[serde(rename = "matches")]
    documents: Vec<QueriedDocument>,
}

impl Client {
    // https://docs.pinecone.io/docs/limits
    const UPSERT_LIMIT: usize = 100;
    const FETCH_AND_DELETE_LIMIT: usize = 1_000;
    const QUERY_LIMIT: usize = 10_000;

    // TODO: remove after testing
    #[allow(dead_code)]
    pub(super) async fn describe_index(&self) -> Result<IndexStats, Error> {
        // https://docs.pinecone.io/reference/describe_index
        self.request(
            Method::POST,
            ["describe_index_stats"],
            NO_PARAMS,
            NO_BODY,
            false,
        )
        .await
        .map_err(Into::into)
    }

    pub(super) async fn upsert_documents(
        &self,
        documents: impl IntoIterator<Item = &IngestedDocument>,
    ) -> Result<Warning<DocumentId>, Error> {
        #[derive(Serialize)]
        struct IngestedDocument<'a> {
            id: &'a DocumentId,
            #[serde(rename = "values")]
            embedding: &'a NormalizedEmbedding,
            #[serde(rename = "sparseValues")]
            sparse_embedding: &'a NormalizedSparseEmbedding,
            #[serde(
                rename = "metadata",
                serialize_with = "DocumentProperties::serialize_time_to_int",
                skip_serializing_if = "HashMap::is_empty"
            )]
            properties: &'a DocumentProperties,
        }

        let mut documents = documents.into_iter().peekable();
        while documents.peek().is_some() {
            let documents = documents
                .by_ref()
                .take(Self::UPSERT_LIMIT)
                .map(
                    |(id, properties, embedding, sparse_embedding)| IngestedDocument {
                        id,
                        embedding,
                        sparse_embedding,
                        properties,
                    },
                )
                .collect_vec();
            let body = json!({ "vectors": documents });

            // https://docs.pinecone.io/reference/upsert
            self.request::<IgnoredResponse>(
                Method::POST,
                ["vectors", "upsert"],
                NO_PARAMS,
                Some(body),
                true,
            )
            .await?;
        }

        // TODO: partial failures error handling possible?
        Ok(Warning::default())
    }

    pub(super) async fn update_documents(
        &self,
        documents: impl IntoIterator<Item = UpdatedDocument<'_>>,
    ) -> Result<Warning<DocumentId>, Error> {
        #[derive(Debug, Serialize)]
        struct UpdatedDocument<'a> {
            id: &'a DocumentId,
            #[serde(rename = "values", skip_serializing_if = "Option::is_none")]
            embedding: Option<&'a NormalizedEmbedding>,
            #[serde(rename = "sparseValues", skip_serializing_if = "Option::is_none")]
            sparse_embedding: Option<&'a NormalizedSparseEmbedding>,
            #[serde(
                rename = "setMetadata",
                serialize_with = "DocumentProperties::serialize_time_to_int",
                skip_serializing_if = "HashMap::is_empty"
            )]
            properties: &'a DocumentProperties,
        }

        for (id, embedding, sparse_embedding, properties) in documents {
            if embedding.is_none() && sparse_embedding.is_none() && properties.is_empty() {
                continue;
            }
            let body = json!(UpdatedDocument {
                id,
                embedding,
                sparse_embedding,
                properties,
            });

            // https://docs.pinecone.io/reference/update
            self.request::<IgnoredResponse>(
                Method::POST,
                ["vectors", "update"],
                NO_PARAMS,
                Some(body),
                true,
            )
            .await?;
        }

        // TODO: partial failures error handling possible?
        Ok(Warning::default())
    }

    async fn get_documents_by_id(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<FetchedDocuments, Error> {
        let mut ids = ids.into_iter().peekable();
        let mut documents = FetchedDocuments::default();
        while ids.peek().is_some() {
            let params = ids
                .by_ref()
                .take(Self::FETCH_AND_DELETE_LIMIT)
                .map(|id| ("ids", Some(&**id)));

            // https://docs.pinecone.io/reference/fetch
            documents.extend(
                self.request::<FetchedDocuments>(
                    Method::GET,
                    ["vectors", "fetch"],
                    params,
                    NO_BODY,
                    true,
                )
                .await?,
            );
        }

        Ok(documents)
    }

    pub(super) async fn get_documents_by_embedding(
        &self,
        params: KnnSearchParams<
            '_,
            impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
        >,
    ) -> Result<HashMap<DocumentId, f32>, Error> {
        // search with `topK` set to zero is a bad request
        if params.count == 0 {
            return Ok(HashMap::new());
        }
        let excluded = params.excluded.into_iter();

        let Value::Object(mut body) = json!({
            "includeValues": "false",
            "includeMetadata": "false",
            "topK": (params.count + excluded.len()).min(Self::QUERY_LIMIT)
        }) else {
            unreachable!(/* body is a json object */);
        };
        if let Some(published_after) = params.published_after {
            body.insert(
                "filter".to_string(),
                json!({
                    DocumentPropertyId::PUBLICATION_DATE: { "$gte": published_after.timestamp() }
                }),
            );
        }
        if let Some(sparse_embedding) = params.sparse_embedding {
            // TODO: parametrize hybrid weight
            body.insert("vector".to_string(), json!(params.embedding * 0.5));
            body.insert("sparseVector".to_string(), json!((sparse_embedding * 0.5)?));
        } else {
            body.insert("vector".to_string(), json!(params.embedding));
        }

        // https://docs.pinecone.io/reference/query
        let mut documents = self
            .request::<QueriedDocuments>(Method::POST, ["query"], NO_PARAMS, Some(body), true)
            .await?
            .documents
            .into_iter()
            .map(|document| (document.id, (document.score + 1.) / 2.))
            .collect::<HashMap<_, _>>();

        for document in excluded {
            documents.remove(document);
        }
        if let Some(min_score) = params.min_similarity {
            documents.retain(|_, score| *score >= min_score);
        }
        if documents.len() > params.count {
            documents = documents
                .into_iter()
                .sorted_unstable_by(|(_, s1), (_, s2)| s1.total_cmp(s2).reverse())
                .collect();
        }

        Ok(documents)
    }

    pub(super) async fn delete_documents(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<Warning<DocumentId>, Error> {
        let mut ids = ids.into_iter().peekable();
        while ids.peek().is_some() {
            let body = json!({
                "deleteAll": "false",
                "ids": ids.by_ref().take(Self::FETCH_AND_DELETE_LIMIT).collect_vec()
            });

            // https://docs.pinecone.io/reference/delete_post
            self.request::<IgnoredResponse>(
                Method::POST,
                ["vectors", "delete"],
                NO_PARAMS,
                Some(body),
                true,
            )
            .await?;
        }

        // TODO: remove partial failures warning
        Ok(Warning::default())
    }

    pub(super) async fn upsert_document_properties(
        &self,
        id: &DocumentId,
        properties: &DocumentProperties,
    ) -> Result<Option<()>, Error> {
        if self
            .get_documents_by_id([id])
            .await?
            .into_iter()
            .next()
            .is_some()
        {
            self.update_documents([(id, None, None, properties)])
                .await
                .map(|_| Some(()))
        } else {
            Ok(None)
        }
    }

    pub(super) async fn delete_document_properties(
        &self,
        id: &DocumentId,
    ) -> Result<Option<()>, Error> {
        if let Some((_, document)) = self.get_documents_by_id([id]).await?.into_iter().next() {
            self.delete_documents([id]).await?;
            self.upsert_documents([&(
                document.id,
                DocumentProperties::default(),
                document.embedding,
                document.sparse_embedding,
            )])
            .await
            .map(|_| Some(()))
        } else {
            Ok(None)
        }
    }

    pub(super) async fn upsert_document_property(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
        property: &DocumentProperty,
    ) -> Result<Option<()>, Error> {
        if self
            .get_documents_by_id([document_id])
            .await?
            .into_iter()
            .next()
            .is_some()
        {
            let properties = [(property_id.clone(), property.clone())]
                .into_iter()
                .collect();

            self.update_documents([(document_id, None, None, &properties)])
                .await
                .map(|_| Some(()))
        } else {
            Ok(None)
        }
    }

    pub(super) async fn delete_document_property(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
    ) -> Result<Option<Option<()>>, Error> {
        if let Some((_, mut document)) = self
            .get_documents_by_id([document_id])
            .await?
            .into_iter()
            .next()
        {
            if document.properties.remove(property_id).is_some() {
                self.delete_documents([document_id]).await?;
                self.upsert_documents([&(
                    document.id,
                    document.properties,
                    document.embedding,
                    document.sparse_embedding,
                )])
                .await?;
            }
            Ok(Some(Some(())))
        } else {
            Ok(None)
        }
    }
}
