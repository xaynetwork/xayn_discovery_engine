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

use derive_more::Deref;
use xayn_web_api_shared::{pinecone, request::TenantId, SetupError};

#[derive(Deref)]
pub(crate) struct Client(pinecone::Client);

impl Client {
    pub(crate) fn builder(config: pinecone::Config) -> Result<ClientBuilder, SetupError> {
        pinecone::Client::new(config).map(ClientBuilder)
    }
}

#[derive(Clone)]
pub(crate) struct ClientBuilder(pinecone::Client);

impl ClientBuilder {
    pub(crate) fn build_for(&self, _tenant_id: &TenantId) -> Client {
        // TODO: multi tenancy
        Client(self.0.clone())
    }
}

// TODO: remove after testing
#[cfg(test)]
mod tests {
    use reqwest::Method;
    use serde_json::json;
    use xayn_ai_bert::{Embedding1, SparseEmbedding};
    use xayn_web_api_shared::url::NO_PARAMS;

    use super::*;
    use crate::{
        models::{DocumentId, DocumentProperties, DocumentPropertyId},
        storage::{utils::IgnoredResponse, KnnSearchParams},
    };

    fn client() -> Client {
        let config = pinecone::Config {
            api_key: "change-me".to_string().into(),
            ..Default::default()
        };

        Client(pinecone::Client::new(config).unwrap())
    }

    #[tokio::test]
    #[ignore = "no local pinecone instance"]
    async fn test_describe_index() {
        let stats = client().describe_index().await.unwrap();
        println!("{stats:#?}");
    }

    #[tokio::test]
    #[ignore = "no local pinecone instance"]
    async fn test_upsert_documents() {
        let docs = vec![
            (
                DocumentId::new("d1").unwrap(),
                [(
                    DocumentPropertyId::PUBLICATION_DATE.try_into().unwrap(),
                    json!("2023-05-14T20:22:50Z").into(),
                )]
                .into_iter()
                .collect(),
                Embedding1::from(vec![1.; 384]).normalize().unwrap(),
                SparseEmbedding::new(vec![0, 127, 10_000], vec![1., 1., 0.5])
                    .unwrap()
                    .normalize()
                    .unwrap(),
            ),
            (
                DocumentId::new("d2").unwrap(),
                DocumentProperties::default(),
                Embedding1::from(vec![-1.; 384]).normalize().unwrap(),
                SparseEmbedding::new(vec![0, 127, 100_000], vec![-1., -1., -0.5])
                    .unwrap()
                    .normalize()
                    .unwrap(),
            ),
        ];
        let warnings = client().upsert_documents(&docs).await.unwrap();
        println!("{warnings:?}");
    }

    #[tokio::test]
    #[ignore = "no local pinecone instance"]
    async fn test_update_documents() {
        let id = &DocumentId::new("d1").unwrap();
        // let embedding = &Embedding1::from(vec![-1.; 384]).normalize().unwrap();
        // let sparse_embedding = &SparseEmbedding::new(vec![0, 42], vec![1., -1.])
        //     .unwrap()
        //     .normalize()
        //     .unwrap();
        // let properties = &[(
        //     DocumentPropertyId::PUBLICATION_DATE.try_into().unwrap(),
        //     json!("2024-05-14T20:22:50Z").into(),
        // )]
        // .into();
        let warnings = client()
            .update_documents([(id, None, None, &DocumentProperties::default())])
            .await
            .unwrap();
        println!("{warnings:?}");
    }

    #[tokio::test]
    #[ignore = "no local pinecone instance"]
    async fn test_get_documents_by_id() {
        let ids = vec![
            DocumentId::new("d1").unwrap(),
            // DocumentId::new("d2").unwrap(),
            // DocumentId::new("d3").unwrap(),
        ];
        let docs = client().get_documents_by_id(&ids).await.unwrap();
        println!("{docs:#?}");
    }

    #[tokio::test]
    #[ignore = "no local pinecone instance"]
    async fn test_get_documents_by_embedding() {
        let embedding = Embedding1::from(vec![1.; 384]).normalize().unwrap();
        let sparse_embedding = SparseEmbedding::new(vec![0, 127, 100_000], vec![-1., -1., -0.5])
            .unwrap()
            .normalize()
            .unwrap();
        let params = KnnSearchParams {
            excluded: [
                // &DocumentId::new("d2").unwrap(),
                // &DocumentId::new("d3").unwrap(),
            ],
            embedding: &embedding,
            sparse_embedding: Some(&sparse_embedding),
            count: 5,
            num_candidates: 10,
            min_similarity: None,
            published_after: None,
            // published_after: chrono::DateTime::parse_from_rfc3339("2023-05-24T20:22:50Z")
            //     .map(Into::into)
            //     .ok(),
            query: None,
        };
        let docs = client().get_documents_by_embedding(params).await.unwrap();
        println!("{docs:#?}");
    }

    #[tokio::test]
    #[ignore = "no local pinecone instance"]
    async fn test_delete_documents() {
        let ids = vec![
            DocumentId::new("d1").unwrap(),
            DocumentId::new("d2").unwrap(),
            DocumentId::new("d3").unwrap(),
        ];
        let warnings = client().delete_documents(&ids).await.unwrap();
        println!("{warnings:?}");
    }

    #[tokio::test]
    #[ignore = "no local pinecone instance"]
    async fn test_delete_all_documents() {
        client()
            .request::<IgnoredResponse>(
                Method::POST,
                ["vectors", "delete"],
                NO_PARAMS,
                Some(json!({ "deleteAll": "true" })),
                true,
            )
            .await
            .unwrap();
    }
}
