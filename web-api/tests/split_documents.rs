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
use url::Url;
use xayn_integration_tests::{
    send_assert,
    send_assert_json,
    test_two_apps,
    with_dev_options,
    UNCHANGED_CONFIG,
};
use xayn_web_api::{Ingestion, Personalization};

#[derive(Debug, Deserialize)]
struct PersonalizedDocumentData {
    id: String,
    score: f32,
    // included by test only dev option
    #[serde(default)]
    splits: HashSet<usize>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    documents: Vec<PersonalizedDocumentData>,
}

impl SearchResponse {
    fn id_ranking(&self) -> Vec<&str> {
        self.documents.iter().map(|doc| &*doc.id).collect_vec()
    }

    fn assert_ordered(&self) {
        let ordered = self
            .documents
            .iter()
            .zip(self.documents.iter().skip(1))
            .all(|(first, second)| first.score > second.score);
        assert!(
            ordered,
            "expects documents to be ordered by score (desc): {:?}",
            self.documents
        );
    }
}

async fn query(
    client: &Client,
    personalization_url: &Url,
    text: &str,
    count: usize,
    filter: Option<Value>,
) -> Result<SearchResponse, Error> {
    let res = send_assert_json::<SearchResponse>(
        client,
        client
            .post(personalization_url.join("/semantic_search")?)
            .json(&json!({
                "document": { "query": text },
                "count": count,
                "filter": filter,
                "_dev": { "include_splits": true }
            }))
            .build()?,
        StatusCode::OK,
        false,
    )
    .await;

    res.assert_ordered();

    Ok(res)
}

async fn set_candidates(client: &Client, ingestion_url: &Url, ids: &[&str]) -> Result<(), Error> {
    let ids = ids.iter().map(|id| json!({ "id": id })).collect_vec();
    send_assert(
        client,
        client
            .put(ingestion_url.join("/documents/_candidates")?)
            .json(&json!({ "documents": ids }))
            .build()?,
        StatusCode::NO_CONTENT,
        false,
    )
    .await;
    Ok(())
}

async fn get_properties(
    client: &Client,
    ingestion_url: &Url,
    id: &str,
) -> Result<Map<String, Value>, Error> {
    let mut url = ingestion_url.clone();
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

async fn ingest(client: &Client, ingestion_url: &Url) -> Result<(), Error> {
    send_assert(
        client,
        client
            .post(ingestion_url.join("/documents")?)
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
                        "snippet": "Cars with wheels. This is a blue sentence. Trains on tracks. ",
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
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        with_dev_options(),
        |client, ingestion_url, personalization_url, _| async move {
            let query = |text, count| query(&client, &personalization_url, text, count, None);
            ingest(&client, &ingestion_url).await?;

            let res = query("cars", 2).await?;
            assert_eq!(res.id_ranking(), vec!["d2"]);
            assert_eq!(res.documents[0].splits, [0, 2].into_iter().collect());

            let res = query("color", 3).await?;
            assert_eq!(res.id_ranking(), vec!["d1", "d2"]);
            assert_eq!(res.documents[0].splits, [0, 1].into_iter().collect());
            assert_eq!(res.documents[1].splits, [1].into_iter().collect());
            Ok(())
        },
    );
}

#[test]
fn test_split_documents_with_set_candidates() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        with_dev_options(),
        |client, ingestion_url, personalization_url, _| async move {
            let query = |text, count| query(&client, &personalization_url, text, count, None);
            let set_candidates = |ids| set_candidates(&client, &ingestion_url, ids);
            ingest(&client, &ingestion_url).await?;

            set_candidates(&[]).await?;
            let res = query("cars", 3).await?;
            assert!(res.documents.is_empty());

            set_candidates(&["d1"]).await?;
            let res = query("arbitrary", 4).await?;
            assert_eq!(res.id_ranking(), vec!["d1"]);
            assert_eq!(res.documents[0].splits, [0, 1, 2].into_iter().collect());

            set_candidates(&["d2"]).await?;
            let res = query("arbitrary", 4).await?;
            assert_eq!(res.id_ranking(), vec!["d2"]);
            assert_eq!(res.documents[0].splits, [0, 1, 2].into_iter().collect());

            set_candidates(&["d1", "d2"]).await?;
            let res = query("color", 3).await?;
            assert_eq!(res.id_ranking(), vec!["d1", "d2"]);
            assert_eq!(res.documents[0].splits, [0, 1].into_iter().collect());
            assert_eq!(res.documents[1].splits, [1].into_iter().collect());
            Ok(())
        },
    );
}

#[test]
fn test_split_documents_with_property_updates() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        with_dev_options(),
        |client, ingestion_url, personalization_url, _| async move {
            let query = |text, value| {
                query(
                    &client,
                    &personalization_url,
                    text,
                    10,
                    Some(json!({
                        "foo": {
                            "$eq": value
                        }
                    })),
                )
            };
            let get_properties = |id| get_properties(&client, &ingestion_url, id);
            ingest(&client, &ingestion_url).await?;

            send_assert(
                &client,
                client
                    .post(ingestion_url.join("/documents/_indexed_properties")?)
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
            assert_eq!(res.id_ranking(), vec!["d2", "d1"]);
            assert_eq!(res.documents[0].splits, [0, 1, 2].into_iter().collect());
            assert_eq!(res.documents[1].splits, [0, 1, 2].into_iter().collect());

            send_assert(
                &client,
                client
                    .put(ingestion_url.join("/documents/d1/properties")?)
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
            assert_eq!(res.id_ranking(), vec!["d2"]);
            assert_eq!(res.documents[0].splits, [0, 1, 2].into_iter().collect());

            send_assert(
                &client,
                client
                    .delete(ingestion_url.join("/documents/d2/properties")?)
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
                    .put(ingestion_url.join("/documents/d1/properties/foo")?)
                    .json(&json!({
                        "property": "filter-a",
                    }))
                    .build()?,
                StatusCode::NO_CONTENT,
                false,
            )
            .await;

            let res = query("cars", "filter-a").await?;
            assert_eq!(res.id_ranking(), vec!["d1"]);
            assert_eq!(res.documents[0].splits, [0, 1, 2].into_iter().collect());

            send_assert(
                &client,
                client
                    .put(ingestion_url.join("/documents/d2/properties/foo")?)
                    .json(&json!({
                        "property": "filter-a",
                    }))
                    .build()?,
                StatusCode::NO_CONTENT,
                false,
            )
            .await;

            let res = query("cars", "filter-a").await?;
            assert_eq!(res.id_ranking(), vec!["d2", "d1"]);
            assert_eq!(res.documents[0].splits, [0, 1, 2].into_iter().collect());
            assert_eq!(res.documents[1].splits, [0, 1, 2].into_iter().collect());

            send_assert(
                &client,
                client
                    .delete(ingestion_url.join("/documents/d1/properties/foo")?)
                    .build()?,
                StatusCode::NO_CONTENT,
                false,
            )
            .await;

            let res = query("cars", "filter-a").await?;
            assert_eq!(res.id_ranking(), vec!["d2"]);
            assert_eq!(res.documents[0].splits, [0, 1, 2].into_iter().collect());

            Ok(())
        },
    );
}

#[test]
fn test_endpoints_which_do_not_yet_fully_support_split_do_not_fall_over() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        with_dev_options(),
        |client, ingestion_url, personalization_url, _| async move {
            ingest(&client, &ingestion_url).await?;
            send_assert(
                &client,
                client
                    .patch(personalization_url.join("/users/u1/interactions")?)
                    .json(&json!({ "documents": [ { "id": "d1" } ] }))
                    .build()?,
                StatusCode::NO_CONTENT,
                false,
            )
            .await;
            send_assert_json::<SearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/users/u1/personalized_documents")?)
                    .json(&json!({}))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;

            send_assert_json::<SearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
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
                    .post(personalization_url.join("/semantic_search")?)
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
                    .post(personalization_url.join("/semantic_search")?)
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
        },
    );
}
