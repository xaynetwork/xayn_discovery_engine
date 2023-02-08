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

use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::json;
use xayn_integration_tests::{send_assert, send_assert_json, test_two_apps, unchanged_config};
use xayn_test_utils::error::Panic;
use xayn_web_api::{Ingestion, Personalization};

#[derive(Serialize)]
struct IngestedDocument {
    id: String,
    snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<serde_json::Value>,
}

async fn ingest(
    client: &Client,
    base_url: &Url,
    documents: &[IngestedDocument],
) -> Result<(), Panic> {
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
    score: f32,
    #[serde(default)]
    properties: serde_json::Value,
}

#[derive(Deserialize)]
struct SemanticSearchResponse {
    documents: Vec<PersonalizedDocumentData>,
}

#[tokio::test]
async fn test_semantic_search() {
    test_two_apps::<Ingestion, Personalization, _>(
        unchanged_config,
        unchanged_config,
        |client, ingestion_url, personalization_url, _services| async move {
            ingest(
                &client,
                &ingestion_url,
                &[
                    IngestedDocument {
                        id: "d1".into(),
                        snippet: "this is one sentence which we have".into(),
                        properties: None,
                    },
                    IngestedDocument {
                        id: "d2".into(),
                        snippet: "duck duck quack".into(),
                        properties: Some(json!({ "dodo": 4 })),
                    },
                    IngestedDocument {
                        id: "d3".into(),
                        snippet: "this is another sentence which we have".into(),
                        properties: None,
                    },
                ],
            )
            .await?;

            let SemanticSearchResponse { documents } = send_assert_json(
                &client,
                client
                    .get(personalization_url.join("/semantic_search/d1")?)
                    .build()?,
                StatusCode::OK,
            )
            .await;

            if let [first, second] = &documents[..] {
                assert_eq!(first.id, "d3");
                assert_eq!(second.id, "d2");
                assert!(first.score > second.score);
                assert!(first.properties.is_null());
                assert_eq!(second.properties, json!({ "dodo": 4 }))
            } else {
                panic!("Unexpected number of documents: {documents:?}");
            }

            Ok(())
        },
    )
    .await;
}

#[tokio::test]
async fn test_semantic_search_min_similarity() {
    test_two_apps::<Ingestion, Personalization, _>(
        unchanged_config,
        unchanged_config,
        |client, ingestion_url, personalization_url, _services| async move {
            ingest(
                &client,
                &ingestion_url,
                &[
                    IngestedDocument {
                        id: "d1".into(),
                        snippet: "Computers are made of technology.".into(),
                        properties: None,
                    },
                    IngestedDocument {
                        id: "d2".into(),
                        snippet: "Mountains smaller than a river.".into(),
                        properties: None,
                    },
                    IngestedDocument {
                        id: "d3".into(),
                        snippet: "Computer technology is made".into(),
                        properties: None,
                    },
                ],
            )
            .await?;

            let SemanticSearchResponse { documents } = send_assert_json(
                &client,
                client
                    .get(personalization_url.join("/semantic_search/d1")?)
                    .query(&[("min_similarity", "0.6")])
                    .build()?,
                StatusCode::OK,
            )
            .await;

            if let [first] = &documents[..] {
                assert_eq!(first.id, "d3");
                assert!(first.score >= 0.6);
            } else {
                panic!("Unexpected number of documents: {documents:?}");
            }

            Ok(())
        },
    )
    .await;
}