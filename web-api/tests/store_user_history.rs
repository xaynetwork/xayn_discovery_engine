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

use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use xayn_integration_tests::{send_assert, send_assert_json, set_config_option, test_two_apps};
use xayn_web_api::{Ingestion, Personalization};

#[derive(Deserialize)]
struct PersonalizedDocumentData {
    id: String,
}

#[derive(Deserialize)]
struct PersonalizedDocumentsResponse {
    documents: Vec<PersonalizedDocumentData>,
}

async fn store_user_history(enabled: bool) {
    test_two_apps::<Ingestion, Personalization, _>(
        |_| {},
        |config| {
            set_config_option! { for config =>
                [personalization]
                store_user_history = enabled;
            }
        },
        |client, ingestion, personalization, _| async move {
            send_assert(
                &client,
                client
                    .post(ingestion.join("/documents").unwrap())
                    .json(&json!({ "documents": [
                { "id": "1", "snippet": "a" },
                { "id": "2", "snippet": "b" },
                { "id": "3", "snippet": "c" },
                { "id": "4", "snippet": "d" },
                { "id": "5", "snippet": "e" }
            ] }))
                    .build()?,
                StatusCode::CREATED,
            )
            .await;

            send_assert(
                &client,
                client
                    .patch(personalization.join("/users/u0/interactions").unwrap())
                    .json(&json!({ "documents": [
                        { "id": "2", "type": "Positive" },
                        { "id": "5", "type": "Positive" },
                    ] }))
                    .build()?,
                StatusCode::NO_CONTENT,
            )
            .await;

            let documents = send_assert_json::<PersonalizedDocumentsResponse>(
                &client,
                client
                    .get(
                        personalization
                            .join("/users/u0/personalized_documents")
                            .unwrap(),
                    )
                    .build()?,
                StatusCode::OK,
            )
            .await;
            let documents = documents
                .documents
                .iter()
                .map(|document| document.id.as_str())
                .collect::<HashSet<_>>();
            if enabled {
                assert_eq!(documents, ["1", "3", "4"].into());
            } else {
                assert_eq!(documents, ["1", "2", "3", "4", "5"].into());
            }

            Ok(())
        },
    )
    .await;
}

#[tokio::test]
async fn test_store_user_history_enabled() {
    store_user_history(true).await;
}

#[tokio::test]
async fn test_store_user_history_disabled() {
    store_user_history(false).await;
}
