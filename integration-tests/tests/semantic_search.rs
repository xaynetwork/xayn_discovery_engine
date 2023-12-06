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
use xayn_integration_tests::{
    send_assert,
    send_assert_json,
    test_app,
    with_dev_options,
    UNCHANGED_CONFIG,
};
use xayn_web_api::WebApi;

async fn ingest(client: &Client, url: &Url) -> Result<(), Error> {
    send_assert(
        client,
        client
            .post(url.join("/documents")?)
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
    properties: Option<serde_json::Value>,
    #[serde(default)]
    snippet: Option<String>,
    #[serde(default)]
    dev: Option<DocumentDevData>,
}

#[derive(Debug, Deserialize)]
struct DocumentDevData {
    pub(crate) raw_scores: Option<RawScores>,
}

#[derive(Debug, Deserialize)]
struct RawScores {
    pub(crate) knn: Option<f32>,
    pub(crate) bm25: Option<f32>,
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
            assert!(d2.score >= -1., $($arg)*);
        }
    };
}

#[test]
fn test_semantic_search() {
    test_app::<WebApi, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        ingest(&client, &url).await?;

        send_assert(
            &client,
            client
                .post(url.join("/semantic_search")?)
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
                .post(url.join("/semantic_search")?)
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
        assert_eq!(documents[0].properties, None);
        assert_eq!(documents[1].properties, Some(json!({ "dodo": 4 })));

        Ok(())
    });
}

#[test]
fn test_semantic_search_with_query() {
    test_app::<WebApi, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        ingest(&client, &url).await?;

        send_assert(
            &client,
            client
                .post(url.join("/semantic_search")?)
                .json(&json!({ "document": { "query": "" } }))
                .build()?,
            StatusCode::BAD_REQUEST,
            false,
        )
        .await;

        let SemanticSearchResponse { documents } = send_assert_json(
            &client,
            client
                .post(url.join("/semantic_search")?)
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
        assert_eq!(documents[0].properties, None);
        assert_eq!(documents[1].properties, None);
        assert_eq!(documents[2].properties, Some(json!({ "dodo": 4 })));

        let SemanticSearchResponse { documents } = send_assert_json(
            &client,
            client
                .post(url.join("/semantic_search")?)
                .json(&json!({
                    "document": { "query": "this is one sentence" },
                    "enable_hybrid_search": true,
                    "include_properties": false
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
        assert!(documents[0].properties.is_none());
        assert!(documents[1].properties.is_none());
        assert!(documents[2].properties.is_none());

        Ok(())
    });
}

#[test]
fn test_semantic_search_with_dev_option_hybrid() {
    test_app::<WebApi, _>(with_dev_options(), |client, url, _| async move {
        ingest(&client, &url).await?;

        send_assert_json::<SemanticSearchResponse>(
            &client,
            client
                .post(url.join("/semantic_search")?)
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
                .post(url.join("/semantic_search")?)
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
                .post(url.join("/semantic_search")?)
                .json(&json!({
                    "document": { "query": "this is one sentence" },
                    "enable_hybrid_search": true,
                    "_dev": { "hybrid": { "customize": {
                        "normalize_knn": "identity",
                        "normalize_bm25": "identity",
                        "merge_fn": { "rrf": { "rank_constant": 60. } }
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
                .post(url.join("/semantic_search")?)
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
                .post(url.join("/semantic_search")?)
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
    });
}

#[test]
fn test_semantic_search_with_dev_option_candidates() {
    test_app::<WebApi, _>(with_dev_options(), |client, url, _| async move {
        ingest(&client, &url).await?;

        let SemanticSearchResponse { documents } = send_assert_json(
            &client,
            client
                .post(url.join("/semantic_search")?)
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
                .post(url.join("/semantic_search")?)
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
                .post(url.join("/semantic_search")?)
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
    });
}

#[test]
fn test_semantic_search_include_snippet() {
    test_app::<WebApi, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        ingest(&client, &url).await?;

        let SemanticSearchResponse { documents } = send_assert_json(
            &client,
            client
                .post(url.join("/semantic_search")?)
                .json(&json!({
                    "document": { "query": "this is one sentence" },
                    "count": 3,
                    "include_snippet": true,
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
        assert_eq!(
            documents[0].snippet.as_deref(),
            Some("this is one sentence which we have")
        );
        assert_eq!(
            documents[1].snippet.as_deref(),
            Some("this is another sentence which we have")
        );
        assert_eq!(documents[2].snippet.as_deref(), Some("duck duck quack"));

        Ok(())
    });
}

#[test]
fn test_semantic_search_with_dev_option_raw_scores() {
    test_app::<WebApi, _>(with_dev_options(), |client, url, _| async move {
        ingest(&client, &url).await?;

        let SemanticSearchResponse { documents } = send_assert_json(
            &client,
            client
                .post(url.join("/semantic_search")?)
                .json(&json!({
                    "document": { "query": "this is one sentence" },
                    "_dev": { "show_raw_scores": true }
                }))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;

        assert!(documents.iter().all(|document| document
            .dev
            .as_ref()
            .unwrap()
            .raw_scores
            .as_ref()
            .unwrap()
            .knn
            .is_some()));

        let SemanticSearchResponse { documents } = send_assert_json(
            &client,
            client
                .post(url.join("/semantic_search")?)
                .json(&json!({
                    "document": { "query": "this is one sentence" },
                    "enable_hybrid_search": true,
                    "count": 100,
                    "_dev": { "show_raw_scores": true }
                }))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;

        let documents_with_bm25 = ["d1", "d3"];
        assert!(documents.iter().all(|document| {
            let raw_scores = document.dev.as_ref().unwrap().raw_scores.as_ref().unwrap();

            raw_scores.knn.is_some()
                && if documents_with_bm25.contains(&document.id.as_str()) {
                    raw_scores.bm25.is_some()
                } else {
                    true
                }
        }));

        Ok(())
    });
}
