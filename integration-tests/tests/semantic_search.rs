// Copyright 2021 Xayn AG
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

use integration_tests::{send_assert, send_assert_json, test_two_apps, unchanged_config};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use xayn_web_api::{Ingestion, Personalization};

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
                            { "id": "d1", "snippet": "this is one sentence which we have" },
                            { "id": "d2", "snippet": "duck duck quack", "properties": { "dodo": 4 } },
                            { "id": "d3", "snippet": "this is another sentence which we have" },
                        ]
                    }))
                    .build()?,
                StatusCode::CREATED,
            )
            .await;

            let SearchResults { documents } = send_assert_json(
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
                panic!("Unexpected number of documents: {:?}", documents);
            }

            Ok(())
        },
    )
    .await;
}

#[derive(Deserialize)]
struct SearchResults {
    documents: Vec<Document>,
}

#[derive(Debug, Deserialize)]
struct Document {
    id: String,
    score: f32,
    #[serde(default)]
    properties: serde_json::Value,
}
