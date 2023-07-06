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

use std::collections::HashSet;

use anyhow::Error;
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use xayn_integration_tests::{send_assert, send_assert_json, test_two_apps, UNCHANGED_CONFIG};
use xayn_web_api::{Ingestion, Personalization};

#[derive(Serialize)]
struct IngestedDocument {
    id: String,
    snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<serde_json::Value>,
}

async fn ingest(client: &Client, base_url: &Url, documents: &Value) -> Result<(), Error> {
    send_assert(
        client,
        client
            .post(base_url.join("/documents")?)
            .json(&json!({ "documents": documents }))
            .build()?,
        StatusCode::CREATED,
    )
    .await;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct PersonalizedDocumentData {
    id: String,
}

#[derive(Deserialize)]
struct SemanticSearchResponse {
    documents: Vec<PersonalizedDocumentData>,
}

impl SemanticSearchResponse {
    fn ids(&self) -> HashSet<&str> {
        self.documents
            .iter()
            .map(|document| document.id.as_str())
            .collect()
    }

    fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}

#[test]
fn test_filter_string() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            send_assert(
                &client,
                client
                    .post(ingestion_url.join("/documents/_indexed_properties")?)
                    .json(&json!({
                        "properties": { "p": { "type": "keyword" }, "q": { "type": "keyword" } }
                    }))
                    .build()?,
                StatusCode::ACCEPTED,
            )
            .await;

            ingest(
                &client,
                &ingestion_url,
                &json!([
                    { "id": "d1", "snippet": "one" },
                    { "id": "d2", "snippet": "two", "properties": { "p": "this" } },
                    { "id": "d3", "snippet": "three", "properties": { "p": "that" } }
                ]),
            )
            .await?;

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({ "document": { "query": "zero" } }))
                    .build()?,
                StatusCode::OK,
            )
            .await;
            assert_eq!(documents.ids(), ["d1", "d2", "d3"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p": { "$eq": "this" } }
                    }))
                    .build()?,
                StatusCode::OK,
            )
            .await;
            assert_eq!(documents.ids(), ["d2"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p": { "$eq": "other" } }
                    }))
                    .build()?,
                StatusCode::OK,
            )
            .await;
            assert!(documents.is_empty());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "q": { "$eq": "this" } }
                    }))
                    .build()?,
                StatusCode::OK,
            )
            .await;
            assert!(documents.is_empty());

            Ok(())
        },
    );
}
