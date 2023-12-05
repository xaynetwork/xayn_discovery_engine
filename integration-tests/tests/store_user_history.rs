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
use toml::toml;
use xayn_integration_tests::{send_assert, send_assert_json, test_app};
use xayn_web_api::WebApi;

#[derive(Deserialize)]
struct PersonalizedDocumentData {
    id: String,
}

#[derive(Deserialize)]
struct PersonalizedDocumentsResponse {
    documents: Vec<PersonalizedDocumentData>,
}

fn store_user_history(enabled: bool) {
    test_app::<WebApi, _>(
        Some(toml! {
            [personalization]
            store_user_history = enabled
        }),
        |client, url, _| async move {
            send_assert(
                &client,
                client
                    .post(url.join("/documents")?)
                    .json(&json!({
                        "documents": [
                            { "id": "1", "snippet": "a" },
                            { "id": "2", "snippet": "b" },
                            { "id": "3", "snippet": "c" },
                            { "id": "4", "snippet": "d" },
                            { "id": "5", "snippet": "e" }
                        ]
                    }))
                    .build()?,
                StatusCode::CREATED,
                false,
            )
            .await;

            send_assert(
                &client,
                client
                    .patch(url.join("/users/u0/interactions")?)
                    .json(&json!({ "documents": [ { "id": "2" }, { "id": "5" } ] }))
                    .build()?,
                StatusCode::NO_CONTENT,
                false,
            )
            .await;

            let documents = send_assert_json::<PersonalizedDocumentsResponse>(
                &client,
                client
                    .post(url.join("/users/u0/recommendations")?)
                    .build()?,
                StatusCode::OK,
                false,
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
    );
}

#[test]
fn test_store_user_history_enabled() {
    store_user_history(true);
}

#[test]
fn test_store_user_history_disabled() {
    store_user_history(false);
}
