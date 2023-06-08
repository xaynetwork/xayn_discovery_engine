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

use anyhow::Error;
use itertools::Itertools;
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;
use serde_json::json;
use xayn_integration_tests::{send_assert, send_assert_json, test_two_apps, UNCHANGED_CONFIG};
use xayn_web_api::{Ingestion, Personalization};

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
    )
    .await;
    Ok(())
}

#[derive(Debug, Deserialize, PartialEq)]
struct PersonalizedDocumentData {
    id: String,
    score: f32,
    #[serde(default)]
    properties: serde_json::Value,
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
) -> Result<Vec<PersonalizedDocumentData>, Error> {
    let mut request = client
        .get(personalization_url.join("/users/u1/personalized_documents")?)
        .query(&[("count", "5")]);
    if let Some(published_after) = published_after {
        request = request.query(&[("published_after", published_after)]);
    }
    let request = request.build()?;

    let error = send_assert_json::<PersonalizedDocumentsResponse>(
        client,
        request.try_clone().unwrap(),
        StatusCode::CONFLICT,
    )
    .await;
    assert_eq!(
        error,
        PersonalizedDocumentsResponse::Error(PersonalizedDocumentsError::NotEnoughInteractions),
    );

    interact(client, personalization_url).await?;

    let documents =
        send_assert_json::<PersonalizedDocumentsResponse>(client, request, StatusCode::OK).await;
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

#[test]
fn test_personalization_all_dates() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            ingest_with_dates(&client, &ingestion_url).await?;
            let documents = personalize(&client, &personalization_url, None).await?;
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
fn test_personalization_limited_dates() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            ingest_with_dates(&client, &ingestion_url).await?;
            let documents =
                personalize(&client, &personalization_url, Some("2022-01-01T00:00:00Z")).await?;
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
fn test_personalization_with_tags() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            ingest_with_tags(&client, &ingestion_url).await?;
            let documents = personalize(&client, &personalization_url, None).await?;
            assert_order!(
                &documents,
                ["d5", "d6", "d1", "d8"],
                "unexpected personalized documents: {documents:?}",
            );
            Ok(())
        },
    );
}
