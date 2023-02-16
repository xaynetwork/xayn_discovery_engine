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

use reqwest::StatusCode;
use serde_json::json;
use xayn_integration_tests::{send_assert, test_app, unchanged_config};
use xayn_web_api::Ingestion;

#[tokio::test]
async fn test_ingestion() {
    test_app::<Ingestion, _>(unchanged_config, |client, url, _service| async move {
        send_assert(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d1", "snippet": "once in a spring there was a fall" },
                        { "id": "d2", "snippet": "fall in a once" }
                    ]
                }))
                .build()?,
            StatusCode::CREATED,
        )
        .await;
        Ok(())
    })
    .await;
}
