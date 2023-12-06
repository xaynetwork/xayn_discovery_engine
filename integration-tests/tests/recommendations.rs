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
use xayn_integration_tests::{send_assert, send_assert_json, test_app, UNCHANGED_CONFIG};
use xayn_web_api::WebApi;

async fn ingest(client: &Client, url: &Url) -> Result<(), Error> {
    send_assert(
        client,
        client
            .post(url.join("/documents")?)
            .json(&json!({
                "documents": [
                    { "id": "d1", "snippet": "Computer", "properties": { "publication_date": "2023-01-12T20:20:20Z" } },
                    { "id": "d2", "snippet": "Technology", "properties": { "publication_date": "2023-01-12T20:20:20Z" } },
                    { "id": "d3", "snippet": "Politic", "properties": { "publication_date": "2023-01-12T20:20:20Z" } },
                    { "id": "d4", "snippet": "Laptop", "properties": { "publication_date": "2023-01-12T20:20:20Z" } },
                    { "id": "d5", "snippet": "Smartphone", "properties": { "publication_date": "2023-01-12T20:20:20Z" } },
                    { "id": "d6", "snippet": "Computer", "properties": { "publication_date": "2021-05-12T20:20:20Z" } },
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

async fn interact(client: &Client, url: &Url) -> Result<(), Error> {
    send_assert(
        client,
        client
            .patch(url.join("/users/u1/interactions")?)
            .json(&json!({ "documents": [ { "id": "d2" }, { "id": "d9" } ] }))
            .build()?,
        StatusCode::NO_CONTENT,
        false,
    )
    .await;
    Ok(())
}

#[derive(Debug, Deserialize, Clone)]
struct PersonalizedDocumentData {
    id: String,
    score: f32,
}

#[derive(Debug, Deserialize)]
struct RecommendationResponse {
    documents: Vec<PersonalizedDocumentData>,
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
            // While score can be any arbitrary value it _currently_ should happen to be in [0;1]
            assert!(1. >= d1.score, $($arg)*);
            assert!(d1.score >= d2.score, $($arg)*);
            assert!(d2.score >= 0., $($arg)*);
        }
    };
}

#[test]
fn test_full_personalization_with_inline_history() {
    test_app::<WebApi, _>(UNCHANGED_CONFIG, |client, url, _services| async move {
        ingest(&client, &url).await?;

        let RecommendationResponse { documents } = send_assert_json(
            &client,
            client
                .post(url.join("/recommendations")?)
                .json(&json!({
                    "count": 5,
                    "personalize": { "user": { "history": [ { "id": "d2" }, { "id": "d9" } ] } }
                }))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;
        assert_order!(
            documents,
            ["d8", "d6", "d1", "d5"],
            "Unexpected fully personalized documents"
        );

        Ok(())
    });
}

#[test]
fn test_full_personalization_with_user_id_that_does_not_exist() {
    test_app::<WebApi, _>(UNCHANGED_CONFIG, |client, url, _services| async move {
        ingest(&client, &url).await?;

        send_assert(
            &client,
            client
                .post(url.join("/recommendations")?)
                .json(&json!({
                    "count": 5,
                    "personalize": { "user": { "id": "u1" } }
                }))
                .build()?,
            StatusCode::CONFLICT,
            false,
        )
        .await;

        Ok(())
    });
}

#[test]
fn test_full_personalization_with_user_id_that_has_two_interactions() {
    test_app::<WebApi, _>(UNCHANGED_CONFIG, |client, url, _services| async move {
        ingest(&client, &url).await?;
        interact(&client, &url).await?;

        let RecommendationResponse { documents } = send_assert_json(
            &client,
            client
                .post(url.join("/recommendations")?)
                .json(&json!({
                    "count": 5,
                    "personalize": { "user": { "id": "u1" } }
                }))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;
        assert_order!(
            documents,
            ["d8", "d6", "d1", "d5"],
            "Unexpected fully personalized documents"
        );

        Ok(())
    });
}
