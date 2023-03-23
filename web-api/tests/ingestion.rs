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
use serde_json::{json, Value};
use xayn_integration_tests::{send_assert, send_assert_json, test_app, unchanged_config};
use xayn_test_utils::error::Panic;
use xayn_web_api::Ingestion;

async fn ingest(client: &Client, url: &Url) -> Result<(), Panic> {
    send_assert(
        client,
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
}

#[derive(Debug, Deserialize, PartialEq)]
enum Kind {
    DocumentNotFound,
    FailedToDeleteSomeDocuments,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Details {
    #[serde(rename = "errors")]
    Delete(Value),
}

#[derive(Deserialize)]
struct Error {
    kind: Kind,
    details: Option<Details>,
}

#[tokio::test]
async fn test_ingestion() {
    test_app::<Ingestion, _>(unchanged_config, |client, url, _| async move {
        ingest(&client, &url).await?;
        send_assert(
            &client,
            client.get(url.join("/documents/d1/properties")?).build()?,
            StatusCode::OK,
        )
        .await;
        send_assert(
            &client,
            client.get(url.join("/documents/d2/properties")?).build()?,
            StatusCode::OK,
        )
        .await;
        let error = send_assert_json::<Error>(
            &client,
            client.get(url.join("/documents/d3/properties")?).build()?,
            StatusCode::BAD_REQUEST,
        )
        .await;
        assert_eq!(error.kind, Kind::DocumentNotFound);
        assert!(error.details.is_none());
        Ok(())
    })
    .await;
}

#[tokio::test]
async fn test_deletion() {
    test_app::<Ingestion, _>(unchanged_config, |client, url, _| async move {
        ingest(&client, &url).await?;
        send_assert(
            &client,
            client.delete(url.join("/documents/d1")?).build()?,
            StatusCode::NO_CONTENT,
        )
        .await;
        let error = send_assert_json::<Error>(
            &client,
            client
                .delete(url.join("/documents")?)
                .json(&json!({ "documents": ["d1", "d2"] }))
                .build()?,
            StatusCode::BAD_REQUEST,
        )
        .await;
        assert_eq!(error.kind, Kind::FailedToDeleteSomeDocuments);
        assert_eq!(
            error.details.unwrap(),
            Details::Delete(json!([ { "id": "d1" } ])),
        );
        Ok(())
    })
    .await;
}
