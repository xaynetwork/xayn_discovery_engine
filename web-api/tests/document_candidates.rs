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
use itertools::Itertools;
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;
use serde_json::{json, Value};
use xayn_integration_tests::{send_assert, send_assert_json, test_app, UNCHANGED_CONFIG};
use xayn_web_api::Ingestion;

async fn ingest(client: &Client, url: &Url) -> Result<(), Error> {
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

#[derive(Deserialize)]
struct DocumentCandidatesResponse {
    documents: Vec<String>,
}

impl DocumentCandidatesResponse {
    fn ids(&self) -> HashSet<&str> {
        self.documents.iter().map(AsRef::as_ref).collect()
    }
}

async fn get(client: &Client, url: &Url) -> Result<DocumentCandidatesResponse, Error> {
    let request = client.get(url.join("/documents/candidates")?).build()?;
    let response = send_assert_json(client, request, StatusCode::OK).await;

    Ok(response)
}

async fn set(client: &Client, url: &Url, ids: impl IntoIterator<Item = &str>) -> Result<(), Error> {
    let request = client
        .put(url.join("/documents/candidates")?)
        .json(&json!({ "documents": ids.into_iter().map(|id| json!({ "id": id })).collect_vec() }))
        .build()?;
    send_assert(client, request, StatusCode::NO_CONTENT).await;

    Ok(())
}

#[test]
fn test_candidates_all() {
    test_app::<Ingestion, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        assert!(get(&client, &url).await?.ids().is_empty());
        ingest(&client, &url).await?;
        assert_eq!(get(&client, &url).await?.ids(), ["d1", "d2", "d3"].into());
        set(&client, &url, ["d1", "d2", "d3"]).await?;
        assert_eq!(get(&client, &url).await?.ids(), ["d1", "d2", "d3"].into());
        Ok(())
    });
}

#[test]
fn test_candidates_some() {
    test_app::<Ingestion, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        assert!(get(&client, &url).await?.ids().is_empty());
        ingest(&client, &url).await?;
        assert_eq!(get(&client, &url).await?.ids(), ["d1", "d2", "d3"].into());
        set(&client, &url, ["d1", "d3"]).await?;
        assert_eq!(get(&client, &url).await?.ids(), ["d1", "d3"].into());
        Ok(())
    });
}

#[test]
fn test_candidates_none() {
    test_app::<Ingestion, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        assert!(get(&client, &url).await?.ids().is_empty());
        ingest(&client, &url).await?;
        assert_eq!(get(&client, &url).await?.ids(), ["d1", "d2", "d3"].into());
        set(&client, &url, None).await?;
        assert!(get(&client, &url).await?.ids().is_empty());
        Ok(())
    });
}

#[test]
fn test_candidates_not_default() {
    test_app::<Ingestion, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        assert!(get(&client, &url).await?.ids().is_empty());
        send_assert(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d1", "snippet": "once in a spring" },
                        { "id": "d2", "snippet": "there was a fall" },
                        { "id": "d3", "snippet": "fall in a once", "is_candidate": false },
                        { "id": "d4", "snippet": "once in a fall", "is_candidate": false }
                    ]
                }))
                .build()?,
            StatusCode::CREATED,
        )
        .await;
        assert_eq!(get(&client, &url).await?.ids(), ["d1", "d2"].into());
        set(&client, &url, ["d2", "d3"]).await?;
        assert_eq!(get(&client, &url).await?.ids(), ["d2", "d3"].into());
        Ok(())
    });
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

#[test]
fn test_candidates_warning() {
    test_app::<Ingestion, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        assert!(get(&client, &url).await?.ids().is_empty());
        ingest(&client, &url).await?;
        assert_eq!(get(&client, &url).await?.ids(), ["d1", "d2", "d3"].into());
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
        assert_eq!(get(&client, &url).await?.ids(), ["d1"].into());
        Ok(())
    });
}

#[test]
fn test_candidates_reingestion() {
    test_app::<Ingestion, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        assert!(get(&client, &url).await?.ids().is_empty());
        send_assert(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d1", "snippet": "once in a spring" },
                        { "id": "d2", "snippet": "there was a fall" },
                        { "id": "d3", "snippet": "fall in a once", "is_candidate": false },
                        { "id": "d4", "snippet": "once in a fall", "is_candidate": false }
                    ]
                }))
                .build()?,
            StatusCode::CREATED,
        )
        .await;
        assert_eq!(get(&client, &url).await?.ids(), ["d1", "d2"].into());

        send_assert(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d1", "snippet": "once in a spring", "default_is_candidate": false },
                        { "id": "d2", "snippet": "there was a fall", "is_candidate": false },
                        { "id": "d3", "snippet": "fall in a once", "default_is_candidate": true },
                        { "id": "d4", "snippet": "once in a fall", "is_candidate": true },
                        { "id": "d5", "snippet": "another sentence", "default_is_candidate": false },
                        { "id": "d6", "snippet": "more sentence", "default_is_candidate": true },
                    ]
                }))
                .build()?,
            StatusCode::CREATED,
        )
        .await;
        assert_eq!(get(&client, &url).await?.ids(), ["d1", "d4", "d6"].into());

        Ok(())
    });
}
