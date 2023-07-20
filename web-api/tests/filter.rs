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
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use xayn_integration_tests::{send_assert, send_assert_json, test_two_apps, UNCHANGED_CONFIG};
use xayn_web_api::{Ingestion, Personalization};

#[derive(Serialize)]
struct IngestedDocument {
    id: String,
    snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<serde_json::Value>,
}

async fn index(client: &Client, url: &Url, properties: Value) -> Result<(), Error> {
    send_assert(
        client,
        client
            .post(url.join("/documents/_indexed_properties")?)
            .json(&json!({ "properties": properties }))
            .build()?,
        StatusCode::ACCEPTED,
        false,
    )
    .await;
    Ok(())
}

async fn ingest(client: &Client, url: &Url, documents: Value) -> Result<(), Error> {
    send_assert(
        client,
        client
            .post(url.join("/documents")?)
            .json(&json!({ "documents": documents }))
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
}

#[derive(Deserialize)]
struct SemanticSearchResponse {
    documents: Vec<PersonalizedDocumentData>,
}

impl SemanticSearchResponse {
    fn ids(&self) -> HashSet<&str> {
        self.documents
            .iter()
            .map(|document| document.id.as_str())
            .collect()
    }

    fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}

#[test]
fn test_filter_boolean() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            index(
                &client,
                &ingestion_url,
                json!({ "p1": { "type": "boolean" }, "p2": { "type": "boolean" } }),
            )
            .await?;
            ingest(
                &client,
                &ingestion_url,
                json!([
                    { "id": "d1", "snippet": "one" },
                    { "id": "d2", "snippet": "two", "properties": { "p1": true } },
                    { "id": "d3", "snippet": "three", "properties": { "p1": false } }
                ]),
            )
            .await?;

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({ "document": { "query": "zero" } }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d1", "d2", "d3"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p1": { "$eq": true } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d2"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p1": { "$eq": false } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d3"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p2": { "$eq": true } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert!(documents.is_empty());

            Ok(())
        },
    );
}

#[test]
fn test_filter_keyword() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            index(
                &client,
                &ingestion_url,
                json!({ "p1": { "type": "keyword" }, "p2": { "type": "keyword" } }),
            )
            .await?;
            ingest(
                &client,
                &ingestion_url,
                json!([
                    { "id": "d1", "snippet": "one" },
                    { "id": "d2", "snippet": "two", "properties": { "p1": "this" } },
                    { "id": "d3", "snippet": "three", "properties": { "p1": "that" } }
                ]),
            )
            .await?;

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({ "document": { "query": "zero" } }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d1", "d2", "d3"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p1": { "$eq": "this" } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d2"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p1": { "$eq": "other" } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert!(documents.is_empty());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p2": { "$eq": "this" } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert!(documents.is_empty());

            Ok(())
        },
    );
}

#[test]
fn test_filter_keyword_array_single() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            index(
                &client,
                &ingestion_url,
                json!({ "p1": { "type": "keyword" }, "p2": { "type": "keyword" } }),
            )
            .await?;
            ingest(
                &client,
                &ingestion_url,
                json!([
                    { "id": "d1", "snippet": "one" },
                    { "id": "d2", "snippet": "two", "properties": { "p1": "this" } },
                    { "id": "d3", "snippet": "three", "properties": { "p1": "that" } }
                ]),
            )
            .await?;

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({ "document": { "query": "zero" } }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d1", "d2", "d3"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p1": { "$in": ["this"] } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d2"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p1": { "$in": ["this", "that", "other"] } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d2", "d3"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p1": { "$in": [] } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert!(documents.is_empty());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p1": { "$in": ["other"] } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert!(documents.is_empty());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p2": { "$in": ["this", "that", "other"] } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert!(documents.is_empty());

            Ok(())
        },
    );
}

#[test]
fn test_filter_keyword_array_multiple() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            index(
                &client,
                &ingestion_url,
                json!({ "p1": { "type": "keyword[]" }, "p2": { "type": "keyword[]" } }),
            )
            .await?;
            ingest(
                &client,
                &ingestion_url,
                json!([
                    { "id": "d1", "snippet": "one" },
                    { "id": "d2", "snippet": "two", "properties": { "p1": ["this", "word"] } },
                    { "id": "d3", "snippet": "three", "properties": { "p1": ["that", "word"] } },
                    { "id": "d4", "snippet": "four", "properties": { "p1": ["other", "words"] } }
                ]),
            )
            .await?;

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({ "document": { "query": "zero" } }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d1", "d2", "d3", "d4"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p1": { "$in": ["the", "word"] } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d2", "d3"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p1": { "$in": ["some", "other", "words"] } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d4"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p1": { "$in": ["this", "that", "other"] } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d2", "d3", "d4"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p1": { "$in": [] } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert!(documents.is_empty());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p1": { "$in": ["some", "thing"] } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert!(documents.is_empty());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p2": { "$in": ["this", "that", "other"] } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert!(documents.is_empty());

            Ok(())
        },
    );
}

#[test]
fn test_filter_combine() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            index(
                &client,
                &ingestion_url,
                json!({ "p1": { "type": "keyword" }, "p2": { "type": "keyword" } }),
            )
            .await?;
            ingest(
                &client,
                &ingestion_url,
                json!([
                    { "id": "d1", "snippet": "one" },
                    { "id": "d2", "snippet": "two", "properties": { "p1": "this", "p2": "word" } },
                    { "id": "d3", "snippet": "three", "properties": { "p1": "that", "p2": "too" } }
                ]),
            )
            .await?;

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({ "document": { "query": "zero" } }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d1", "d2", "d3"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": {
                            "$and": [{ "p1": { "$eq": "this" } }, { "p2": { "$eq": "word" } }]
                        }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d2"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": {
                            "$and": [{ "p1": { "$eq": "this" } }, { "p2": { "$eq": "too" } }]
                        }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert!(documents.is_empty());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": {
                            "$or": [{ "p1": { "$eq": "that" } }, { "p2": { "$eq": "word" } }]
                        }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d2", "d3"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": {
                            "$or": [{ "p1": { "$eq": "foo" } }, { "p2": { "$eq": "bar" } }]
                        }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert!(documents.is_empty());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({ "document": { "query": "zero" }, "filter": { "$and": [] } }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d1", "d2", "d3"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({ "document": { "query": "zero" }, "filter": { "$or": [] } }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d1", "d2", "d3"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "$and": [
                            { "$or": [{ "p1": { "$eq": "this" } }, { "p1": { "$eq": "that" } }] },
                            { "$or": [{ "p2": { "$eq": "too" } }, { "p2": { "$eq": "foo" } }] }
                        ] }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d3"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "$or": [
                            { "$and": [{ "p1": { "$eq": "this" } }, { "p2": { "$eq": "word" } }] },
                            { "$and": [{ "p1": { "$eq": "that" } }, { "p2": { "$eq": "too" } }] }
                        ] }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d2", "d3"].into());

            Ok(())
        },
    );
}

#[test]
fn test_filter_date() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            index(
                &client,
                &ingestion_url,
                json!({ "p1": { "type": "date" }, "p2": { "type": "date" } }),
            )
            .await?;
            ingest(
                &client,
                &ingestion_url,
                json!([
                    { "id": "d1", "snippet": "one" },
                    {
                        "id": "d2",
                        "snippet": "two",
                        "properties": { "p1": "1234-05-06T07:08:09Z" }
                    },
                    {
                        "id": "d3",
                        "snippet": "three",
                        "properties": { "p1": "9876-05-04T03:02:01Z" }
                    }
                ]),
            )
            .await?;

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({ "document": { "query": "zero" } }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d1", "d2", "d3"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p1": { "$gt": "2000-01-01T00:00:00Z" } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d3"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "$and": [
                            { "p1": { "$gte": "0000-01-01T00:00:00Z" } },
                            { "p1": { "$lte": "2000-01-01T00:00:00Z" } }
                        ] }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d2"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "$or": [
                            { "p1": { "$lt": "2000-01-01T00:00:00Z" } },
                            { "p1": { "$gt": "9000-01-01T00:00:00Z" } }
                        ] }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(documents.ids(), ["d2", "d3"].into());

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "filter": { "p2": { "$gt": "0000-01-01T00:00:00Z" } }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert!(documents.is_empty());

            Ok(())
        },
    );
}

#[test]
fn test_deprecated_published_after() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, _| async move {
            ingest(
                &client,
                &ingestion_url,
                json!([{
                    "id": "d1",
                    "snippet": "one",
                    "properties": { "publication_date": "1234-05-06T07:08:09Z" }
                }]),
            )
            .await?;

            let documents = send_assert_json::<SemanticSearchResponse>(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "zero" },
                        "published_after": "0000-01-01T00:00:00Z"
                    }))
                    .build()?,
                StatusCode::OK,
                true,
            )
            .await;
            assert_eq!(documents.ids(), ["d1"].into());

            Ok(())
        },
    );
}
