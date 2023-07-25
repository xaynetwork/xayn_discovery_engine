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
use toml::toml;
use xayn_integration_tests::{send_assert, send_assert_json, test_two_apps, UNCHANGED_CONFIG};
use xayn_web_api::{Ingestion, Personalization};

async fn ingest(client: &Client, ingestion_url: &Url) -> Result<(), Error> {
    send_assert(
        client,
        client
            .post(ingestion_url.join("/documents")?)
            .json(&json!({
                "documents": [
                    { "id": "d1", "snippet": "this is one sentence which we have" },
                    { "id": "d2", "snippet": "duck duck quack", "properties": { "dodo": 4 } },
                    { "id": "d3", "snippet": "this is another sentence which we have" }
                ]
            }))
            .build()?,
        StatusCode::CREATED,
        false,
    )
    .await;

    Ok(())
}

#[derive(Debug, Deserialize)]
struct PersonalizedDocumentData {
    id: String,
    score: f32,
    #[serde(default)]
    properties: serde_json::Value,
}

#[derive(Deserialize)]
struct SemanticSearchResponse {
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
            assert!(1. >= d1.score, $($arg)*);
            assert!(d1.score > d2.score, $($arg)*);
            assert!(d2.score >= 0., $($arg)*);
        }
    };
}

#[test]
fn test_semantic_search() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            ingest(&client, &ingestion_url).await?;

            send_assert(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "id": "d1" },
                        "_dev": { "max_number_candidates": 100 }
                    }))
                    .build()?,
                StatusCode::FORBIDDEN,
                false,
            )
            .await;

            let SemanticSearchResponse { documents } = send_assert_json(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({ "document": { "id": "d1" } }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_order!(
                documents,
                ["d3", "d2"],
                "unexpected documents: {documents:?}",
            );
            assert!(documents[0].properties.is_null());
            assert_eq!(documents[1].properties, json!({ "dodo": 4 }));

            Ok(())
        },
    );
}

#[test]
fn test_semantic_search_with_query() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            ingest(&client, &ingestion_url).await?;

            send_assert(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({ "document": { "query": "" } }))
                    .build()?,
                StatusCode::BAD_REQUEST,
                false,
            )
            .await;

            let SemanticSearchResponse { documents } = send_assert_json(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "this is one sentence" },
                        "enable_hybrid_search": true,
                        "include_properties": true
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_order!(
                documents,
                ["d1", "d3", "d2"],
                "unexpected documents: {documents:?}",
            );
            assert!(documents[0].properties.is_null());
            assert!(documents[1].properties.is_null());
            assert_eq!(documents[2].properties, json!({ "dodo": 4 }));

            Ok(())
        },
    );
}

#[test]
fn test_semantic_search_with_dev_option_hybrid() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        Some(toml! {
            [tenants]
            enable_dev = true
        }),
        |client, ingestion_url, personalization_url, _| async move {
            ingest(&client, &ingestion_url).await?;

            send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "this is one sentence" },
                        "enable_hybrid_search": true,
                        "_dev": { "hybrid": { "customize": {
                            "normalize_knn": "identity",
                            "normalize_bm25": "normalize_if_max_gt1",
                            "merge_fn": { "sum": {} }
                        } } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;

            send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "this is one sentence" },
                        "enable_hybrid_search": true,
                        "_dev": { "hybrid": { "customize": {
                            "normalize_knn": "normalize",
                            "normalize_bm25": "normalize",
                            "merge_fn": { "sum": {} }
                        } } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;

            send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "this is one sentence" },
                        "enable_hybrid_search": true,
                        "_dev": { "hybrid": { "customize": {
                            "normalize_knn": "identity",
                            "normalize_bm25": "identity",
                            "merge_fn": { "rrf": { "k": 60. } }
                        } } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;

            send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "this is one sentence" },
                        "enable_hybrid_search": true,
                        "_dev": { "hybrid": { "customize": {
                            "normalize_knn": "identity",
                            "normalize_bm25": "identity",
                            "merge_fn": { "rrf": {} }
                        } } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;

            send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "this is one sentence" },
                        "enable_hybrid_search": true,
                        "_dev": { "hybrid": { "customize": {
                            "normalize_knn": "identity",
                            "normalize_bm25": "identity",
                            "merge_fn": { "rrf": {
                                "knn_weight": 0.8,
                                "bm25_weight": 0.2
                            } }
                        } } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;

            Ok(())
        },
    );
}

#[test]
fn test_semantic_search_with_dev_option_candidates() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        Some(toml! {
            [tenants]
            enable_dev = true
        }),
        |client, ingestion_url, personalization_url, _| async move {
            ingest(&client, &ingestion_url).await?;

            let SemanticSearchResponse { documents } = send_assert_json(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "this is one sentence" },
                        "count": 1,
                        "_dev": { "max_number_candidates": 3 }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_order!(documents, ["d1"], "unexpected documents: {documents:?}");

            let SemanticSearchResponse { documents } = send_assert_json(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "this is one sentence" },
                        "count": 2,
                        "_dev": { "max_number_candidates": 3 }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_order!(
                documents,
                ["d1", "d3"],
                "unexpected documents: {documents:?}",
            );

            let SemanticSearchResponse { documents } = send_assert_json(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "this is one sentence" },
                        "count": 3,
                        "_dev": { "max_number_candidates": 3 }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_order!(
                documents,
                ["d1", "d3", "d2"],
                "unexpected documents: {documents:?}",
            );

            Ok(())
        },
    );
}
