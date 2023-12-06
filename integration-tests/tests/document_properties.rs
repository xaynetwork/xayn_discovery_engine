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

use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{json, Value};
use xayn_integration_tests::{send_assert, send_assert_json, test_app, UNCHANGED_CONFIG};
use xayn_web_api::WebApi;

#[derive(Debug, Deserialize)]
struct DocumentPropertiesResponse {
    properties: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "kind")]
enum Error {
    DocumentNotFound,
    DocumentPropertyNotFound,
}

fn document_properties(is_candidate: bool) {
    test_app::<WebApi, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        send_assert(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d1", "snippet": "snippet one", "is_candidate": is_candidate }
                    ]
                }))
                .build()?,
            StatusCode::CREATED,
            false,
        )
        .await;

        let DocumentPropertiesResponse { properties } = send_assert_json(
            &client,
            client.get(url.join("/documents/d1/properties")?).build()?,
            StatusCode::OK,
            false,
        )
        .await;
        assert!(properties.is_empty());
        let error = send_assert_json::<Error>(
            &client,
            client.get(url.join("/documents/d2/properties")?).build()?,
            StatusCode::BAD_REQUEST,
            false,
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
            false,
        )
        .await;
        let error = send_assert_json::<Error>(
            &client,
            client
                .put(url.join("/documents/d2/properties")?)
                .json(&json!({ "properties": {} }))
                .build()?,
            StatusCode::BAD_REQUEST,
            false,
        )
        .await;
        assert_eq!(error, Error::DocumentNotFound);
        let DocumentPropertiesResponse { properties } = send_assert_json(
            &client,
            client.get(url.join("/documents/d1/properties")?).build()?,
            StatusCode::OK,
            false,
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
            false,
        )
        .await;
        let error = send_assert_json::<Error>(
            &client,
            client
                .delete(url.join("/documents/d2/properties")?)
                .build()?,
            StatusCode::BAD_REQUEST,
            false,
        )
        .await;
        assert_eq!(error, Error::DocumentNotFound);
        let DocumentPropertiesResponse { properties } = send_assert_json(
            &client,
            client.get(url.join("/documents/d1/properties")?).build()?,
            StatusCode::OK,
            false,
        )
        .await;
        assert!(properties.is_empty());

        Ok(())
    });
}

#[test]
fn test_document_properties_candidate() {
    document_properties(true);
}

#[test]
fn test_document_properties_noncandidate() {
    document_properties(false);
}

#[derive(Debug, Deserialize)]
struct DocumentPropertyResponse {
    property: Value,
}

fn document_property(is_candidate: bool) {
    test_app::<WebApi, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        send_assert(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d1", "snippet": "snippet one", "is_candidate": is_candidate }
                    ]
                }))
                .build()?,
            StatusCode::CREATED,
            false,
        )
        .await;

        let error = send_assert_json::<Error>(
            &client,
            client
                .get(url.join("/documents/d1/properties/p1")?)
                .build()?,
            StatusCode::BAD_REQUEST,
            false,
        )
        .await;
        assert_eq!(error, Error::DocumentPropertyNotFound);
        let error = send_assert_json::<Error>(
            &client,
            client
                .get(url.join("/documents/d2/properties/p1")?)
                .build()?,
            StatusCode::BAD_REQUEST,
            false,
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
            false,
        )
        .await;
        let error = send_assert_json::<Error>(
            &client,
            client
                .put(url.join("/documents/d2/properties/p1")?)
                .json(&json!({ "property": 42 }))
                .build()?,
            StatusCode::BAD_REQUEST,
            false,
        )
        .await;
        assert_eq!(error, Error::DocumentNotFound);
        let DocumentPropertyResponse { property } = send_assert_json(
            &client,
            client
                .get(url.join("/documents/d1/properties/p1")?)
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;
        assert_eq!(property, json!(42));

        send_assert(
            &client,
            client
                .delete(url.join("/documents/d1/properties/p1")?)
                .build()?,
            StatusCode::NO_CONTENT,
            false,
        )
        .await;
        let error = send_assert_json::<Error>(
            &client,
            client
                .delete(url.join("/documents/d1/properties/p1")?)
                .build()?,
            StatusCode::BAD_REQUEST,
            false,
        )
        .await;
        assert_eq!(error, Error::DocumentPropertyNotFound);
        let error = send_assert_json::<Error>(
            &client,
            client
                .delete(url.join("/documents/d2/properties/p1")?)
                .build()?,
            StatusCode::BAD_REQUEST,
            false,
        )
        .await;
        assert_eq!(error, Error::DocumentNotFound);

        Ok(())
    });
}

#[test]
fn test_document_property_candidate() {
    document_property(true);
}

#[test]
fn test_document_property_noncandidate() {
    document_property(false);
}
