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

use chrono::Utc;
use itertools::Itertools;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use xayn_integration_tests::{send_assert, send_assert_json, test_two_apps, unchanged_config};
use xayn_web_api::{Ingestion, Personalization};

#[derive(Debug, Deserialize)]
struct PersonalizedDocumentData {
    id: String,
    #[allow(dead_code)]
    score: f32,
    #[allow(dead_code)]
    properties: serde_json::Value,
}

#[derive(Deserialize)]
struct StatelessPersonalizedDocumentsResponse {
    documents: Vec<PersonalizedDocumentData>,
}

#[tokio::test]
async fn test_test_app() {
    test_two_apps::<Ingestion, Personalization, _>(
        unchanged_config,
        unchanged_config,
        |client, ingestion_url, personalization_url, _services| async move {
            send_assert(
                &client,
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
                        ]
                    }))
                    .build()?,
                StatusCode::CREATED,
            )
            .await;

            let StatelessPersonalizedDocumentsResponse {documents} = send_assert_json(
                &client,
                client
                    .post(personalization_url.join("/personalized_documents")?)
                    .json(&json!({
                        "published_after": "2022-05-12T20:20:20Z",
                        "history": [
                            { "id": "d1", "timestamp": Utc::now().to_rfc3339() },
                            { "id": "d4" },
                            { "id": "d5", "timestamp": null }
                        ]
                    }))
                    .build()?,
                StatusCode::OK,
            )
            .await;

            let found_ids = documents.iter().map(|document| &document.id).collect_vec();
            assert_eq!(found_ids, &["d2", "d3"], "unexpected documents {documents:?}");
            Ok(())
        },
    )
    .await;
}
