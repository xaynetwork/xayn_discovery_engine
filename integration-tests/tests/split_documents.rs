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
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use toml::toml;
use url::Url;
use xayn_integration_tests::{send_assert, send_assert_json, test_app, with_dev_options};
use xayn_web_api::WebApi;

#[derive(Debug, Deserialize)]
struct PersonalizedDocumentData {
    id: String,
    snippet_id: SnippetId,
    score: f32,
    // included by dev option
    #[serde(default)]
    snippet: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SnippetId {
    document_id: String,
    sub_id: u32,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    documents: Vec<PersonalizedDocumentData>,
}

impl SearchResponse {
    fn id_ranking(&self) -> Vec<(&str, u32)> {
        self.documents
            .iter()
            .map(|document| {
                (
                    &*document.snippet_id.document_id,
                    document.snippet_id.sub_id,
                )
            })
            .collect_vec()
    }

    fn sanity_checks(&self) {
        let ordered = self
            .documents
            .iter()
            .tuple_windows()
            .all(|(first, second)| first.score > second.score);
        assert!(
            ordered,
            "expects documents to be ordered by score (desc): {:?}",
            self.documents
        );

        for document in &self.documents {
            assert_eq!(&document.id, &document.snippet_id.document_id);
        }
    }
}

async fn query(
    client: &Client,
    url: &Url,
    text: &str,
    count: usize,
    filter: Option<Value>,
) -> Result<SearchResponse, Error> {
    let res = send_assert_json::<SearchResponse>(
        client,
        client
            .post(url.join("/semantic_search")?)
            .json(&json!({
                "document": { "query": text },
                "count": count,
                "filter": filter,
                "include_snippet": true,
            }))
            .build()?,
        StatusCode::OK,
        false,
    )
    .await;

    res.sanity_checks();

    Ok(res)
}

async fn set_candidates(client: &Client, url: &Url, ids: &[&str]) -> Result<(), Error> {
    let ids = ids.iter().map(|id| json!({ "id": id })).collect_vec();
    send_assert(
        client,
        client
            .put(url.join("/documents/_candidates")?)
            .json(&json!({ "documents": ids }))
            .build()?,
        StatusCode::NO_CONTENT,
        false,
    )
    .await;
    Ok(())
}

async fn get_properties(client: &Client, url: &Url, id: &str) -> Result<Map<String, Value>, Error> {
    let mut url = url.clone();
    url.path_segments_mut()
        .unwrap()
        .extend(["documents", id, "properties"]);

    #[derive(Deserialize)]
    struct Response {
        properties: Map<String, Value>,
    }

    Ok(
        send_assert_json::<Response>(client, client.get(url).build()?, StatusCode::OK, false)
            .await
            .properties,
    )
}

fn json_map(
    pairs: impl IntoIterator<Item = (impl Into<String>, impl Into<Value>)>,
) -> Map<String, Value> {
    pairs
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect()
}

async fn ingest(client: &Client, url: &Url) -> Result<(), Error> {
    send_assert(
        client,
        client
            .post(url.join("/documents")?)
            .json(&json!({
                "documents": [
                    {
                        "id": "d1",
                        "snippet": "The duck is blue. The truck is yellow. But birds are birds.",
                        "split": true,
                        "properties": {
                            "foo": "filter-a",
                        }
                    },
                    {
                        "id": "d2",
                        "snippet": "Cars with wheels. This is a blue sentence. Trains on tracks.",
                        "split": true,
                        "properties": {
                            "foo": "filter-a",
                        }
                    },
                ]
            }))
            .build()?,
        StatusCode::CREATED,
        false,
    )
    .await;

    Ok(())
}

#[test]
fn test_split_documents_for_semantic_search() {
    test_app::<WebApi, _>(with_dev_options(), |client, url, _| async move {
        let query = |text, count| query(&client, &url, text, count, None);
        ingest(&client, &url).await?;

        let res = query("cars", 2).await?;
        assert_eq!(res.id_ranking(), vec![("d2", 0), ("d2", 2)]);
        assert_eq!(
            res.documents[0].snippet.as_deref(),
            Some("Cars with wheels.")
        );
        assert_eq!(
            res.documents[1].snippet.as_deref(),
            Some("Trains on tracks.")
        );

        let res = query("color", 3).await?;
        assert_eq!(res.id_ranking(), vec![("d1", 0), ("d1", 1), ("d2", 1)]);
        assert_eq!(
            res.documents[0].snippet.as_deref(),
            Some("The duck is blue.")
        );
        assert_eq!(
            res.documents[1].snippet.as_deref(),
            Some("The truck is yellow.")
        );
        assert_eq!(
            res.documents[2].snippet.as_deref(),
            Some("This is a blue sentence.")
        );
        Ok(())
    });
}

#[test]
fn test_split_documents_with_set_candidates() {
    test_app::<WebApi, _>(with_dev_options(), |client, url, _| async move {
        let query = |text, count| query(&client, &url, text, count, None);
        let set_candidates = |ids| set_candidates(&client, &url, ids);
        ingest(&client, &url).await?;

        set_candidates(&[]).await?;
        let res = query("cars", 3).await?;
        assert!(res.documents.is_empty());

        set_candidates(&["d1"]).await?;
        let res = query("arbitrary", 4).await?;
        assert_eq!(
            res.id_ranking().into_iter().collect::<HashSet<_>>(),
            [("d1", 0), ("d1", 1), ("d1", 2)].into()
        );

        set_candidates(&["d2"]).await?;
        let res = query("arbitrary", 4).await?;
        assert_eq!(
            res.id_ranking().into_iter().collect::<HashSet<_>>(),
            [("d2", 0), ("d2", 1), ("d2", 2)].into()
        );

        set_candidates(&["d1", "d2"]).await?;
        let res = query("color", 3).await?;
        assert_eq!(
            res.id_ranking().into_iter().collect::<HashSet<_>>(),
            [("d1", 0), ("d1", 1), ("d2", 1)].into()
        );
        Ok(())
    });
}

#[test]
fn test_split_documents_with_property_updates() {
    test_app::<WebApi, _>(with_dev_options(), |client, url, _| async move {
        let query = |text, value| {
            query(
                &client,
                &url,
                text,
                10,
                Some(json!({
                    "foo": {
                        "$eq": value
                    }
                })),
            )
        };
        let get_properties = |id| get_properties(&client, &url, id);
        ingest(&client, &url).await?;

        send_assert(
            &client,
            client
                .post(url.join("/documents/_indexed_properties")?)
                .json(&json!({
                    "properties": {
                        "foo": {
                            "type": "keyword"
                        }
                    }
                }))
                .build()?,
            StatusCode::ACCEPTED,
            false,
        )
        .await;

        let res = query("cars", "filter-a").await?;
        assert_eq!(
            res.id_ranking(),
            vec![
                ("d2", 0),
                ("d2", 2),
                ("d1", 1),
                ("d2", 1),
                ("d1", 2),
                ("d1", 0)
            ]
        );

        send_assert(
            &client,
            client
                .put(url.join("/documents/d1/properties")?)
                .json(&json!({
                    "properties": {
                        "foo": "filter-b"
                    }
                }))
                .build()?,
            StatusCode::NO_CONTENT,
            false,
        )
        .await;

        assert_eq!(get_properties("d1").await?, json_map([("foo", "filter-b")]));
        assert_eq!(get_properties("d2").await?, json_map([("foo", "filter-a")]));

        let res = query("cars", "filter-a").await?;
        assert_eq!(res.id_ranking(), vec![("d2", 0), ("d2", 2), ("d2", 1),]);

        send_assert(
            &client,
            client
                .delete(url.join("/documents/d2/properties")?)
                .build()?,
            StatusCode::NO_CONTENT,
            false,
        )
        .await;

        let res = query("cars", "filter-a").await?;
        assert!(res.id_ranking().is_empty());

        send_assert(
            &client,
            client
                .put(url.join("/documents/d1/properties/foo")?)
                .json(&json!({
                    "property": "filter-a",
                }))
                .build()?,
            StatusCode::NO_CONTENT,
            false,
        )
        .await;

        let res = query("cars", "filter-a").await?;
        assert_eq!(res.id_ranking(), vec![("d1", 1), ("d1", 2), ("d1", 0),]);

        send_assert(
            &client,
            client
                .put(url.join("/documents/d2/properties/foo")?)
                .json(&json!({
                    "property": "filter-a",
                }))
                .build()?,
            StatusCode::NO_CONTENT,
            false,
        )
        .await;

        let res = query("cars", "filter-a").await?;
        assert_eq!(
            res.id_ranking(),
            vec![
                ("d2", 0),
                ("d2", 2),
                ("d1", 1),
                ("d2", 1),
                ("d1", 2),
                ("d1", 0)
            ]
        );

        send_assert(
            &client,
            client
                .delete(url.join("/documents/d1/properties/foo")?)
                .build()?,
            StatusCode::NO_CONTENT,
            false,
        )
        .await;

        let res = query("cars", "filter-a").await?;
        assert_eq!(res.id_ranking(), vec![("d2", 0), ("d2", 2), ("d2", 1),]);

        let res = send_assert_json::<SearchResponse>(
            &client,
            client
                .post(url.join("/semantic_search")?)
                .json(&json!({
                    "document": {
                        "id": {
                            "document_id": "d2",
                            "sub_id": 1
                        }
                    },
                    "count": 2,
                }))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;

        assert_eq!(res.id_ranking(), vec![("d1", 0), ("d1", 2)]);

        Ok(())
    });
}

#[test]
fn test_split_allows_huge_snippets() {
    test_app::<WebApi, _>(
        Some(toml! {
            [ingestion]
            max_snippet_size = 3
        }),
        |client, url, _| async move {
            send_assert(
                &client,
                client
                    .post(url.join("/documents")?)
                    .json(&json!({
                        "documents": [
                            {
                                "id": "d1",
                                "snippet": "Too long.",
                                "properties": {
                                    "foo": "filter-a",
                                }
                            },
                        ]
                    }))
                    .build()?,
                StatusCode::BAD_REQUEST,
                false,
            )
            .await;

            send_assert(
                &client,
                client
                    .post(url.join("/documents")?)
                    .json(&json!({
                        "documents": [
                            {
                                "id": "d1",
                                "snippet": "Too long.",
                                "split": true,
                                "properties": {
                                    "foo": "filter-a",
                                }
                            },
                        ]
                    }))
                    .build()?,
                StatusCode::CREATED,
                false,
            )
            .await;
            Ok(())
        },
    )
}

#[test]
fn test_endpoints_which_do_not_yet_fully_support_split_do_not_fall_over() {
    test_app::<WebApi, _>(with_dev_options(), |client, url, _| async move {
        ingest(&client, &url).await?;
        send_assert(
            &client,
            client
                .patch(url.join("/users/u1/interactions")?)
                .json(&json!({ "documents": [ { "id": "d1" } ] }))
                .build()?,
            StatusCode::NO_CONTENT,
            false,
        )
        .await;
        send_assert_json::<SearchResponse>(
            &client,
            client
                .post(url.join("/users/u1/recommendations")?)
                .json(&json!({}))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;

        send_assert_json::<SearchResponse>(
            &client,
            client
                .post(url.join("/semantic_search")?)
                .json(&json!({
                    "document": {
                        "query": "car"
                    },
                    "personalize": {
                        "user": {
                            "id": "u1"
                        }
                    },
                }))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;

        send_assert_json::<SearchResponse>(
            &client,
            client
                .post(url.join("/semantic_search")?)
                .json(&json!({
                    "document": {
                        "query": "car"
                    },
                    "personalize": {
                        "user": {
                            "history": [ { "id": "d1" }]
                        }
                    },
                }))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;

        send_assert_json::<SearchResponse>(
            &client,
            client
                .post(url.join("/semantic_search")?)
                .json(&json!({
                    "document": {
                        "query": "car"
                    },
                    "personalize": {
                        "user": {
                            "history": [ { "id":  { "document_id": "d1", "sub_id": 2 } } ]
                        }
                    },
                }))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;

        send_assert_json::<SearchResponse>(
            &client,
            client
                .post(url.join("/semantic_search")?)
                .json(&json!({
                    "document": {
                        "id": "d1"
                    },
                }))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;

        Ok(())
    });
}
