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

use itertools::Itertools;
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;
use serde_json::json;
use xayn_integration_tests::{send_assert, send_assert_json, test_two_apps, unchanged_config};
use xayn_test_utils::error::Panic;
use xayn_web_api::{Ingestion, Personalization};

async fn ingest_documents(client: &Client, ingestion_url: &Url) -> Result<(), Panic> {
    send_assert(
        client,
        client
            .post(ingestion_url.join("/documents")?)
            .json(&json!({
                "documents": [
                    { "id": "d1", "snippet": "Computer", "properties": { "publication_date": "2023-01-12T20:20:20Z" } },
                    { "id": "d2", "snippet": "Technology", "properties": { "publication_date": "2023-01-12T20:20:20Z" } },
                    { "id": "d3", "snippet": "Politic", "properties": { "publication_date": "2023-01-12T20:20:20Z" } },
                    { "id": "d4", "snippet": "Laptop", "properties": { "publication_date": "2023-01-12T20:20:20Z" } },
                    { "id": "d5", "snippet": "Smartphone", "properties": { "publication_date": "2023-01-12T20:20:20Z" } },
                    { "id": "d6", "snippet": "Computer", "properties": { "publication_date": "2021-05-12T20:20:20Z" } },
                    { "id": "d7", "snippet": "Dogs" },
                    { "id": "d8", "snippet": "Chicken" },
                    { "id": "d9", "snippet": "Robot Chicken" }
                ]
            }))
            .build()?,
        StatusCode::CREATED,
    )
    .await;
    Ok(())
}

#[derive(Deserialize)]
struct Error {
    kind: String,
}

#[tokio::test]
async fn test_not_enough_interactions() {
    test_two_apps::<Ingestion, Personalization, _>(
        unchanged_config,
        unchanged_config,
        |client, ingestion_url, personalization_url, _services| async move {
            ingest_documents(&client, &ingestion_url).await?;

            let error =
                send_assert_json::<Error>(
                    &client,
                    client
                        .get(personalization_url.join(
                            "/semantic_search/d1?personalization_ratio=1.0&personalize_for=u1",
                        )?)
                        .build()?,
                    StatusCode::CONFLICT,
                )
                .await;

            assert_eq!(error.kind, "NotEnoughInteractions");
            Ok(())
        },
    )
    .await;
}

#[derive(Debug, Deserialize)]
struct PersonalizedDocumentData {
    id: String,
    #[allow(dead_code)]
    score: f32,
    #[allow(dead_code)]
    #[serde(default)]
    properties: serde_json::Value,
}

#[derive(Deserialize)]
struct SemanticSearchResponse {
    documents: Vec<PersonalizedDocumentData>,
}

impl SemanticSearchResponse {
    fn ids(&self) -> Vec<&str> {
        self.documents
            .iter()
            .map(|document| document.id.as_str())
            .collect_vec()
    }
}

#[tokio::test]
async fn test_personalization() {
    test_two_apps::<Ingestion, Personalization, _>(
        unchanged_config,
        unchanged_config,
        |client, ingestion_url, personalization_url, _services| async move {
            ingest_documents(&client, &ingestion_url).await?;
            send_assert(
                &client,
                client
                    .patch(personalization_url.join("/users/u1/interactions")?)
                    .json(&json!({
                        "documents": [
                            { "id": "d2", "type": "Positive" },
                            { "id": "d9", "type": "Positive" }
                        ]
                    }))
                    .build()?,
                StatusCode::NO_CONTENT,
            )
            .await;

            let not_personalized = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .get(personalization_url.join("/semantic_search/d1?count=5")?)
                    .build()?,
                StatusCode::OK,
            )
            .await;
            assert_eq!(
                not_personalized.ids(),
                ["d6", "d4", "d2", "d5", "d7"],
                "unexpected not personalized documents: {:?}",
                not_personalized.documents,
            );

            let fully_personalized = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .get(personalization_url.join(
                        "/semantic_search/d1?count=5&personalize_ratio=1.0&personalize_for=u1",
                    )?)
                    .build()?,
                StatusCode::OK,
            )
            .await;
            assert_eq!(
                fully_personalized.ids(),
                ["d6", "d8", "d5", "d4", "d7"],
                "unexpected fully personalized documents: {:?}",
                fully_personalized.documents,
            );

            let subtle_personalized = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .get(personalization_url.join(
                        "/semantic_search/d1?count=5&personalize_ratio=0.1&personalize_for=u1",
                    )?)
                    .build()?,
                StatusCode::OK,
            )
            .await;
            assert_eq!(
                subtle_personalized.ids(),
                ["d6", "d4", "d5", "d8", "d7"],
                "unexpected subtle personalized documents: {:?}",
                subtle_personalized.documents,
            );

            Ok(())
        },
    )
    .await;
}
