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

use std::collections::HashMap;

use anyhow::Error;
use itertools::Itertools;
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;
use serde_json::{json, Value};
use xayn_integration_tests::{send_assert, send_assert_json, test_two_apps, UNCHANGED_CONFIG};
use xayn_web_api::{Ingestion, Personalization};
use xayn_web_api_shared::serde::json_object;

async fn ingest_with_dates(client: &Client, ingestion_url: &Url) -> Result<(), Error> {
    send_assert(
        client,
        client
            .post(ingestion_url.join("/documents")?)
            .json(&json!({
                "documents": [
                    { "id": "d1", "snippet": "Computer", "properties": { "publication_date": "2023-01-12T20:20:20Z" } },
                    { "id": "d2", "snippet": "Technology", "properties": { "publication_date": "2023-05-12T20:20:20Z" } },
                    { "id": "d3", "snippet": "Politic", "properties": { "publication_date": "2023-02-12T20:20:20Z" } },
                    { "id": "d4", "snippet": "Laptop", "properties": { "publication_date": "2100-01-01T00:00:00Z" } },
                    { "id": "d5", "snippet": "Smartphone", "properties": { "publication_date": "2023-08-12T20:20:20Z" } },
                    { "id": "d6", "snippet": "Computers", "properties": { "publication_date": "2021-05-12T20:20:20Z" } },
                    { "id": "d7", "snippet": "Dogs" },
                    { "id": "d8", "snippet": "Chicken" },
                    { "id": "d9", "snippet": "Robot Chicken" }
                ]
            }))
            .build()?,
        StatusCode::CREATED,
        false,
    )
    .await;
    Ok(())
}

async fn ingest_with_tags(client: &Client, ingestion_url: &Url) -> Result<(), Error> {
    send_assert(
        client,
        client
            .post(ingestion_url.join("/documents")?)
            .json(&json!({
                "documents": [
                    { "id": "d1", "snippet": "Computer", "tags": ["tec"] },
                    { "id": "d2", "snippet": "Technology", "tags": ["tec", "soc"] },
                    { "id": "d3", "snippet": "Politic", "tags": ["soc"] },
                    { "id": "d4", "snippet": "Laptop", "tags": ["tec"] },
                    { "id": "d5", "snippet": "Smartphone", "tags": ["tec", "soc"] },
                    { "id": "d6", "snippet": "Computers", "tags": ["tec"] },
                    { "id": "d7", "snippet": "Dogs" ,"tags": ["nat"] },
                    { "id": "d8", "snippet": "Chicken", "tags": ["nat"] },
                    { "id": "d9", "snippet": "Robot Chicken", "tags": ["nat", "tec"] }
                ]
            }))
            .build()?,
        StatusCode::CREATED,
        false,
    )
    .await;
    Ok(())
}

async fn interact(client: &Client, personalization_url: &Url) -> Result<(), Error> {
    send_assert(
        client,
        client
            .patch(personalization_url.join("/users/u1/interactions")?)
            .json(&json!({ "documents": [ { "id": "d2" }, { "id": "d9" } ] }))
            .build()?,
        StatusCode::NO_CONTENT,
        false,
    )
    .await;
    Ok(())
}

#[derive(Debug, Deserialize, PartialEq)]
struct PersonalizedDocumentData {
    id: String,
    score: f32,
    #[serde(default)]
    properties: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, PartialEq)]
enum PersonalizedDocumentsError {
    NotEnoughInteractions,
}

#[derive(Debug, Deserialize, PartialEq)]
enum PersonalizedDocumentsResponse {
    #[serde(rename = "documents")]
    Documents(Vec<PersonalizedDocumentData>),
    #[serde(rename = "kind")]
    Error(PersonalizedDocumentsError),
}

async fn personalize(
    client: &Client,
    personalization_url: &Url,
    published_after: Option<&str>,
    include_properties: Option<bool>,
    is_deprecated: bool,
) -> Result<Vec<PersonalizedDocumentData>, Error> {
    let request = if is_deprecated {
        let mut request = client
            .get(personalization_url.join("/users/u1/personalized_documents")?)
            .query(&[("count", "5")]);
        if let Some(published_after) = published_after {
            request = request.query(&[(
                "filter",
                json!({ "publication_date": { "$gte": published_after } }).to_string(),
            )]);
        }
        if let Some(include_properties) = include_properties {
            request = request.query(&[("include_properties", &include_properties.to_string())]);
        }
        request
    } else {
        let mut body = json_object!({ "count": 5 });
        if let Some(published_after) = published_after {
            body.insert(
                "filter".into(),
                json!({ "publication_date": { "$gte": published_after } }),
            );
        }
        if let Some(include_properties) = include_properties {
            body.insert("include_properties".into(), json!(include_properties));
        }
        client
            .post(personalization_url.join("/users/u1/personalized_documents")?)
            .json(&body)
    }
    .build()?;

    let error = send_assert_json::<PersonalizedDocumentsResponse>(
        client,
        request.try_clone().unwrap(),
        StatusCode::CONFLICT,
        is_deprecated,
    )
    .await;
    assert_eq!(
        error,
        PersonalizedDocumentsResponse::Error(PersonalizedDocumentsError::NotEnoughInteractions),
    );

    interact(client, personalization_url).await?;

    let documents = send_assert_json::<PersonalizedDocumentsResponse>(
        client,
        request,
        StatusCode::OK,
        is_deprecated,
    )
    .await;
    assert!(matches!(
        documents,
        PersonalizedDocumentsResponse::Documents(_),
    ));
    let PersonalizedDocumentsResponse::Documents(documents) = documents else {
        unreachable!();
    };

    Ok(documents)
}

macro_rules! assert_order {
    ($documents: expr, $ids: expr, $($arg: tt)*) => {
        assert_eq!(
            $documents
                .iter()
                .map(|document| document.id.as_str())
                .collect_vec(),
            $ids,
            $($arg)*
        );
        for documents in $documents.windows(2) {
            let [d1, d2] = documents else { unreachable!() };
            assert!(1. >= d1.score, $($arg)*);
            assert!(d1.score > d2.score, $($arg)*);
            assert!(d2.score >= 0., $($arg)*);
        }
    };
}

fn personalization_all_dates(is_deprecated: bool) {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            ingest_with_dates(&client, &ingestion_url).await?;
            let documents =
                personalize(&client, &personalization_url, None, None, is_deprecated).await?;
            assert_order!(
                &documents,
                ["d8", "d6", "d1", "d5"],
                "unexpected personalized documents: {documents:?}",
            );
            Ok(())
        },
    );
}

#[test]
fn test_personalization_all_dates() {
    personalization_all_dates(false);
}

#[test]
fn test_deprecated_personalization_all_dates() {
    personalization_all_dates(true);
}

fn personalization_limited_dates(is_deprecated: bool) {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            ingest_with_dates(&client, &ingestion_url).await?;
            let documents = personalize(
                &client,
                &personalization_url,
                Some("2022-01-01T00:00:00Z"),
                None,
                is_deprecated,
            )
            .await?;
            assert_order!(
                &documents,
                ["d1", "d5", "d4", "d3"],
                "unexpected personalized documents: {documents:?}",
            );
            Ok(())
        },
    );
}

#[test]
fn test_personalization_limited_dates() {
    personalization_limited_dates(false);
}

#[test]
fn test_deprecated_personalization_limited_dates() {
    personalization_limited_dates(true);
}

fn personalization_with_tags(is_deprecated: bool) {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            ingest_with_tags(&client, &ingestion_url).await?;
            let documents =
                personalize(&client, &personalization_url, None, None, is_deprecated).await?;
            assert_order!(
                &documents,
                ["d5", "d6", "d1", "d8"],
                "unexpected personalized documents: {documents:?}",
            );
            Ok(())
        },
    );
}

#[test]
fn test_personalization_with_tags() {
    personalization_with_tags(false);
}

#[test]
fn test_deprecated_personalization_with_tags() {
    personalization_with_tags(true);
}

fn personalization_include_properties(include_properties: bool, is_deprecated: bool) {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            ingest_with_dates(&client, &ingestion_url).await?;
            let documents = personalize(
                &client,
                &personalization_url,
                None,
                Some(include_properties),
                is_deprecated,
            )
            .await?;
            let is_empty = documents
                .iter()
                .map(|document| document.properties.is_empty())
                .collect_vec();
            if include_properties {
                assert_eq!(is_empty, [true, false, false, false]);
            } else {
                assert_eq!(is_empty, [true, true, true, true]);
            }
            Ok(())
        },
    );
}

#[test]
fn test_personalization_include_properties() {
    personalization_include_properties(true, false);
}

#[test]
fn test_personalization_exclude_properties() {
    personalization_include_properties(false, false);
}

#[test]
fn test_deprecated_personalization_include_properties() {
    personalization_include_properties(true, true);
}

#[test]
fn test_deprecated_personalization_exclude_properties() {
    personalization_include_properties(false, true);
}

#[test]
fn test_personalize_no_body() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            ingest_with_dates(&client, &ingestion_url).await?;
            send_assert(
                &client,
                client
                    .post(personalization_url.join("/users/u1/personalized_documents")?)
                    .build()?,
                StatusCode::CONFLICT,
                false,
            )
            .await;
            Ok(())
        },
    );
}
