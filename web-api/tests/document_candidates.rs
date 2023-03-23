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

#![cfg(feature = "ET-4089")]

use itertools::Itertools;
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;
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
                    { "id": "d2", "snippet": "fall in a once" },
                    { "id": "d3", "snippet": "once in a fall" }
                ]
            }))
            .build()?,
        StatusCode::CREATED,
    )
    .await;
    Ok(())
}

async fn set(client: &Client, url: &Url, ids: impl IntoIterator<Item = &str>) -> Result<(), Panic> {
    let request = client
        .put(url.join("/documents/candidates")?)
        .json(&json!({ "documents": ids.into_iter().map(|id| json!({ "id": id })).collect_vec() }))
        .build()?;
    send_assert(client, request, StatusCode::NO_CONTENT).await;

    Ok(())
}

#[tokio::test]
async fn test_candidates_all() {
    test_app::<Ingestion, _>(unchanged_config, |client, url, _| async move {
        ingest(&client, &url).await?;
        set(&client, &url, ["d1", "d2", "d3"]).await?;
        Ok(())
    })
    .await;
}

#[tokio::test]
async fn test_candidates_some() {
    test_app::<Ingestion, _>(unchanged_config, |client, url, _| async move {
        ingest(&client, &url).await?;
        set(&client, &url, ["d1", "d3"]).await?;
        Ok(())
    })
    .await;
}

#[tokio::test]
async fn test_candidates_none() {
    test_app::<Ingestion, _>(unchanged_config, |client, url, _| async move {
        ingest(&client, &url).await?;
        set(&client, &url, None).await?;
        Ok(())
    })
    .await;
}

#[derive(Debug, Deserialize, PartialEq)]
enum Kind {
    DocumentNotFound,
    FailedToSetSomeDocumentCandidates,
}

#[derive(Debug, Deserialize, PartialEq)]
enum Details {
    #[serde(rename = "documents")]
    Set(Value),
}

#[derive(Deserialize)]
struct Error {
    kind: Kind,
    details: Details,
}

#[tokio::test]
async fn test_candidates_warning() {
    test_app::<Ingestion, _>(unchanged_config, |client, url, _| async move {
        ingest(&client, &url).await?;
        let error = send_assert_json::<Error>(
            &client,
            client
                .put(url.join("/documents/candidates")?)
                .json(&json!({
                    "documents": [
                        { "id": "d1" },
                        { "id": "d4" }
                    ]
                }))
                .build()?,
            StatusCode::BAD_REQUEST,
        )
        .await;
        assert_eq!(error.kind, Kind::FailedToSetSomeDocumentCandidates);
        assert_eq!(error.details, Details::Set(json!([ { "id": "d4" } ])));
        Ok(())
    })
    .await;
}
