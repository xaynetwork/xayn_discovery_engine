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

#[cfg(feature = "ET-3837")]
use std::collections::HashMap;

#[cfg(feature = "ET-3837")]
use reqwest::StatusCode;
#[cfg(feature = "ET-3837")]
use serde::Deserialize;
#[cfg(feature = "ET-3837")]
use serde_json::{json, Value};
#[cfg(feature = "ET-3837")]
use xayn_integration_tests::{send_assert, send_assert_json, test_app, unchanged_config};
#[cfg(feature = "ET-3837")]
use xayn_web_api::Ingestion;

#[cfg(feature = "ET-3837")]
#[derive(Debug, Deserialize)]
struct DocumentPropertiesResponse {
    properties: HashMap<String, Value>,
}

#[cfg(feature = "ET-3837")]
#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "kind")]
enum Error {
    DocumentNotFound,
    DocumentPropertyNotFound,
}

#[cfg(feature = "ET-3837")]
#[tokio::test]
async fn test_document_properties() {
    test_app::<Ingestion, _>(unchanged_config, |client, url, _| async move {
        send_assert(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d1", "snippet": "snippet one" }
                    ]
                }))
                .build()?,
            StatusCode::CREATED,
        )
        .await;

        let DocumentPropertiesResponse { properties } = send_assert_json(
            &client,
            client.get(url.join("/documents/d1/properties")?).build()?,
            StatusCode::OK,
        )
        .await;
        assert!(properties.is_empty());
        let error = send_assert_json::<Error>(
            &client,
            client.get(url.join("/documents/d2/properties")?).build()?,
            StatusCode::BAD_REQUEST,
        )
        .await;
        assert_eq!(error, Error::DocumentNotFound);

        send_assert(
            &client,
            client
                .put(url.join("/documents/d1/properties")?)
                .json(&json!({ "properties": { "some": "thing", "else": 42 } }))
                .build()?,
            StatusCode::NO_CONTENT,
        )
        .await;
        let error = send_assert_json::<Error>(
            &client,
            client
                .put(url.join("/documents/d2/properties")?)
                .json(&json!({ "properties": {} }))
                .build()?,
            StatusCode::BAD_REQUEST,
        )
        .await;
        assert_eq!(error, Error::DocumentNotFound);
        let DocumentPropertiesResponse { properties } = send_assert_json(
            &client,
            client.get(url.join("/documents/d1/properties")?).build()?,
            StatusCode::OK,
        )
        .await;
        assert_eq!(
            properties,
            [
                ("some".to_string(), json!("thing")),
                ("else".to_string(), json!(42)),
            ]
            .into(),
        );

        send_assert(
            &client,
            client
                .delete(url.join("/documents/d1/properties")?)
                .build()?,
            StatusCode::NO_CONTENT,
        )
        .await;
        let error = send_assert_json::<Error>(
            &client,
            client
                .delete(url.join("/documents/d2/properties")?)
                .build()?,
            StatusCode::BAD_REQUEST,
        )
        .await;
        assert_eq!(error, Error::DocumentNotFound);
        let DocumentPropertiesResponse { properties } = send_assert_json(
            &client,
            client.get(url.join("/documents/d1/properties")?).build()?,
            StatusCode::OK,
        )
        .await;
        assert!(properties.is_empty());

        Ok(())
    })
    .await;
}

#[cfg(feature = "ET-3837")]
#[derive(Debug, Deserialize)]
struct DocumentPropertyResponse {
    property: Value,
}

#[cfg(feature = "ET-3837")]
#[tokio::test]
async fn test_document_property() {
    test_app::<Ingestion, _>(unchanged_config, |client, url, _| async move {
        send_assert(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d1", "snippet": "snippet one" }
                    ]
                }))
                .build()?,
            StatusCode::CREATED,
        )
        .await;

        let error = send_assert_json::<Error>(
            &client,
            client
                .get(url.join("/documents/d1/properties/p1")?)
                .build()?,
            StatusCode::BAD_REQUEST,
        )
        .await;
        assert_eq!(error, Error::DocumentPropertyNotFound);
        let error = send_assert_json::<Error>(
            &client,
            client
                .get(url.join("/documents/d2/properties/p1")?)
                .build()?,
            StatusCode::BAD_REQUEST,
        )
        .await;
        assert_eq!(error, Error::DocumentNotFound);
        send_assert(
            &client,
            client
                .put(url.join("/documents/d1/properties/p1")?)
                .json(&json!({ "property": 42 }))
                .build()?,
            StatusCode::NO_CONTENT,
        )
        .await;
        let error = send_assert_json::<Error>(
            &client,
            client
                .put(url.join("/documents/d2/properties/p1")?)
                .json(&json!({ "property": 42 }))
                .build()?,
            StatusCode::BAD_REQUEST,
        )
        .await;
        assert_eq!(error, Error::DocumentNotFound);
        let DocumentPropertyResponse { property } = send_assert_json(
            &client,
            client
                .get(url.join("/documents/d1/properties/p1")?)
                .build()?,
            StatusCode::OK,
        )
        .await;
        assert_eq!(property, json!(42));

        send_assert(
            &client,
            client
                .delete(url.join("/documents/d1/properties/p1")?)
                .build()?,
            StatusCode::NO_CONTENT,
        )
        .await;
        let error = send_assert_json::<Error>(
            &client,
            client
                .delete(url.join("/documents/d1/properties/p1")?)
                .build()?,
            StatusCode::BAD_REQUEST,
        )
        .await;
        assert_eq!(error, Error::DocumentPropertyNotFound);
        let error = send_assert_json::<Error>(
            &client,
            client
                .delete(url.join("/documents/d2/properties/p1")?)
                .build()?,
            StatusCode::BAD_REQUEST,
        )
        .await;
        assert_eq!(error, Error::DocumentNotFound);

        Ok(())
    })
    .await;
}
