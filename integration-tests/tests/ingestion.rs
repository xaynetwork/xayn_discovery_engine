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

use std::collections::{HashMap, HashSet};

use base64::{engine::general_purpose, Engine as _};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use toml::toml;
use xayn_integration_tests::{
    send_assert,
    send_assert_json,
    test_app,
    with_text_extractor_options,
    UNCHANGED_CONFIG,
};
use xayn_web_api::WebApi;

async fn ingest(client: &Client, url: &Url) -> Result<(), anyhow::Error> {
    send_assert(
        client,
        client
            .post(url.join("/documents")?)
            .json(&json!({
                "documents": [
                    { "id": "d1", "snippet": "once in a spring there was a fall" },
                    { "id": "d2", "snippet": "fall in a once" }
                ]
            }))
            .build()?,
        StatusCode::CREATED,
        false,
    )
    .await;
    Ok(())
}

#[derive(Debug, Deserialize, PartialEq)]
enum Kind {
    FileUploadNotEnabled,
    DocumentNotFound,
    FailedToValidateDocuments,
    FailedToDeleteSomeDocuments,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Details {
    #[serde(rename = "documents")]
    Ingest(Vec<Value>),
    #[serde(rename = "errors")]
    Delete(Value),
}

#[derive(Deserialize)]
struct Error {
    kind: Kind,
    details: Option<Details>,
}

#[test]
fn test_ingestion_created() {
    test_app::<WebApi, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        ingest(&client, &url).await?;
        send_assert(
            &client,
            client.get(url.join("/documents/d1/properties")?).build()?,
            StatusCode::OK,
            false,
        )
        .await;
        send_assert(
            &client,
            client.get(url.join("/documents/d2/properties")?).build()?,
            StatusCode::OK,
            false,
        )
        .await;
        let error = send_assert_json::<Error>(
            &client,
            client.get(url.join("/documents/d3/properties")?).build()?,
            StatusCode::BAD_REQUEST,
            false,
        )
        .await;
        assert_eq!(error.kind, Kind::DocumentNotFound);
        assert!(error.details.is_none());
        Ok(())
    });
}

#[test]
fn test_ingestion_bad_request() {
    test_app::<WebApi, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        let long_snippet = vec!["a"; 2049].join("");
        let error = send_assert_json::<Error>(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d!", "snippet": "once in a spring there was a fall" },
                        { "id": "d2", "snippet": "fall in a once" },
                        { "id": "d3", "snippet":  long_snippet },
                    ]
                }))
                .build()?,
            StatusCode::BAD_REQUEST,
            false,
        )
        .await;
        assert_eq!(error.kind, Kind::FailedToValidateDocuments);
        let Some(Details::Ingest(failed_documents)) = error.details else {
            panic!("Unexpected error details {:?}", error.details);
        };

        let failed_documents = failed_documents
            .into_iter()
            .map(|mut value| {
                let id = value
                    .as_object_mut()
                    .and_then(|obj| obj.remove("id"))
                    .and_then(|id| {
                        if let Value::String(id) = id {
                            Some(id)
                        } else {
                            None
                        }
                    })
                    .expect("unexpected error format");
                (id, value)
            })
            .collect::<HashMap<_, _>>();

        assert_eq!(
            failed_documents["d!"],
            json!({
                "kind": "InvalidDocumentId",
                "details": {
                    "syntax": {
                        "expected": "^[a-zA-Z0-9\\-:@.][a-zA-Z0-9\\-:@._]*$"
                    }
                }
            })
        );
        assert_eq!(
            failed_documents["d3"],
            json!({
                "kind": "InvalidDocumentSnippet" ,
                "details": {
                    "invalid_string": {
                        "size": {
                            "got": 2049,
                            "bounds": "1..=2048",
                        }
                    }
                }
            })
        );

        let er = send_assert_json::<Error>(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d", "file": general_purpose::STANDARD.encode("once in a spring there was a fall".as_bytes()) },
                    ]
                }))
                .build()?,
            StatusCode::BAD_REQUEST,
            false,
        )
        .await;
        assert_eq!(er.kind, Kind::FileUploadNotEnabled);

        send_assert(
            &client,
            client.get(url.join("/documents/d2/properties")?).build()?,
            StatusCode::OK,
            false,
        )
        .await;
        Ok(())
    });
}

#[test]
fn test_ingestion_created_with_file() {
    let txt_content = "once in a spring there was a fall";
    let html_content = "<!DOCTYPE html>
    <html>
      <head>
        <meta charset=\"utf-8\">
        <title>MarkSheet</title>
        <meta name=\"description\" content=\"A simple HTML and CSS tutorial\">
      </head>
      <body>
        <p>Hello World!</p>
      </body>
    </html>";

    let allowed_content_type = ["application/pdf", "text/plain", "text/html"]
        .into_iter()
        .map(Into::into)
        .collect();

    test_app::<WebApi, _>(
        with_text_extractor_options(allowed_content_type, None),
        |client, url, _| async move {
            send_assert(
                &client,
                client
                    .post(url.join("/documents")?)
                    .json(&json!({
                        "documents": [
                            { "id": "d1", "file": general_purpose::STANDARD.encode(txt_content)},
                            { "id": "d2", "file": general_purpose::STANDARD.encode(html_content)},
                        ]
                    }))
                    .build()?,
                StatusCode::CREATED,
                false,
            )
            .await;

            send_assert(
                &client,
                client.get(url.join("/documents/d1/properties")?).build()?,
                StatusCode::OK,
                false,
            )
            .await;
            send_assert(
                &client,
                client.get(url.join("/documents/d2/properties")?).build()?,
                StatusCode::OK,
                false,
            )
            .await;

            Ok(())
        },
    );
}

#[test]
fn test_ingestion_created_with_file_bad_request() {
    let txt_content_data = general_purpose::STANDARD.encode("once in a spring there was a fall");
    let html_content_empty = general_purpose::STANDARD.encode(
        "<!DOCTYPE html>
    <html>
      <head>
      </head>
      <body>
      </body>
    </html>",
    );

    let txt_content = txt_content_data.clone();
    test_app::<WebApi, _>(
        with_text_extractor_options(vec![], None),
        |client, url, _| async move {
            send_assert(
                &client,
                client
                    .post(url.join("/documents")?)
                    .json(&json!({
                        "documents": [
                            { "id": "d1", "file": txt_content, "summarize": true},
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
                            { "id": "d1", "file": txt_content, "split": false},
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
                            { "id": "d1", "file": "invalid base 64"},
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
                            { "id": "d1", "file": ""},
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
                            { "id": "d1", "file": html_content_empty},
                        ]
                    }))
                    .build()?,
                StatusCode::BAD_REQUEST,
                false,
            )
            .await;

            Ok(())
        },
    );

    let txt_content = txt_content_data;
    test_app::<WebApi, _>(
        with_text_extractor_options(vec![], Some(1)),
        |client, url, _| async move {
            send_assert(
                &client,
                client
                    .post(url.join("/documents")?)
                    .json(&json!({
                        "documents": [
                            { "id": "d1", "file": txt_content },
                        ]
                    }))
                    .build()?,
                StatusCode::BAD_REQUEST,
                false,
            )
            .await;

            Ok(())
        },
    );
}

#[test]
fn test_deletion() {
    test_app::<WebApi, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        ingest(&client, &url).await?;
        send_assert(
            &client,
            client.delete(url.join("/documents/d1")?).build()?,
            StatusCode::NO_CONTENT,
            false,
        )
        .await;
        let error = send_assert_json::<Error>(
            &client,
            client
                .delete(url.join("/documents")?)
                .json(&json!({ "documents": ["d1", "d2"] }))
                .build()?,
            StatusCode::BAD_REQUEST,
            false,
        )
        .await;
        assert_eq!(error.kind, Kind::FailedToDeleteSomeDocuments);
        assert_eq!(
            error.details.unwrap(),
            Details::Delete(json!([{ "id": "d1" }])),
        );
        Ok(())
    });
}

#[derive(Deserialize)]
struct PersonalizedDocumentData {
    id: String,
}

#[derive(Deserialize)]
struct SemanticSearchResponse {
    documents: Vec<PersonalizedDocumentData>,
}

#[test]
fn test_reingestion_candidates() {
    test_app::<WebApi, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        send_assert(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d1", "snippet": "snippet 1", "is_candidate": true },
                        { "id": "d2", "snippet": "snippet 2", "is_candidate": true },
                        { "id": "d3", "snippet": "snippet 3", "is_candidate": false },
                        { "id": "d4", "snippet": "snippet 4", "is_candidate": false }
                    ]
                }))
                .build()?,
            StatusCode::CREATED,
            false,
        )
        .await;
        let SemanticSearchResponse { documents } = send_assert_json(
            &client,
            client
                .post(url.join("/semantic_search")?)
                .json(&json!({
                    "document": { "query": "snippet" },
                    "enable_hybrid_search": true
                }))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;
        assert_eq!(
            documents
                .iter()
                .map(|document| document.id.as_str())
                .collect::<HashSet<_>>(),
            ["d1", "d2"].into(),
        );

        send_assert(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d1", "snippet": "snippet 1", "is_candidate": true },
                        { "id": "d2", "snippet": "snippet 2", "is_candidate": false },
                        { "id": "d3", "snippet": "snippet 3", "is_candidate": true },
                        { "id": "d4", "snippet": "snippet 4", "is_candidate": false }
                    ]
                }))
                .build()?,
            StatusCode::CREATED,
            false,
        )
        .await;
        let SemanticSearchResponse { documents } = send_assert_json(
            &client,
            client
                .post(url.join("/semantic_search")?)
                .json(&json!({
                    "document": { "query": "snippet" },
                    "enable_hybrid_search": true
                }))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;
        assert_eq!(
            documents
                .iter()
                .map(|document| document.id.as_str())
                .collect::<HashSet<_>>(),
            ["d1", "d3"].into(),
        );

        Ok(())
    });
}

// currently there is no endpoint to actually check the changed snippets/embeddings, but we can at
// least run the test to see if something crashes and manually check with log level `info` how many
// new and changed documents have been logged and manually check the databases
#[test]
fn test_reingestion_snippets() {
    test_app::<WebApi, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        send_assert(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d1", "snippet": "snippet 1", "is_candidate": true },
                        { "id": "d2", "snippet": "snippet 2", "is_candidate": true },
                        { "id": "d3", "snippet": "snippet 3", "is_candidate": false },
                        { "id": "d4", "snippet": "snippet 4", "is_candidate": false }
                    ]
                }))
                .build()?,
            StatusCode::CREATED,
            false,
        )
        .await;
        send_assert(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d1", "snippet": "snippet 1", "is_candidate": true },
                        { "id": "d2", "snippet": "snippet X", "is_candidate": true },
                        { "id": "d3", "snippet": "snippet 3", "is_candidate": false },
                        { "id": "d4", "snippet": "snippet Y", "is_candidate": false }
                    ]
                }))
                .build()?,
            StatusCode::CREATED,
            false,
        )
        .await;

        Ok(())
    });
}

#[derive(Debug, Deserialize)]
struct OrderPropertyResponse {
    property: usize,
}

#[test]
fn test_ingestion_same_id() {
    test_app::<WebApi, _>(UNCHANGED_CONFIG, |client, url, _| async move {
        send_assert(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d1", "snippet": "snippet 1", "properties": { "order": 1 } },
                        { "id": "d2", "snippet": "snippet 2", "properties": { "order": 2 } },
                        { "id": "d1", "snippet": "snippet 3", "properties": { "order": 3 } }
                    ]
                }))
                .build()?,
            StatusCode::CREATED,
            false,
        )
        .await;
        let OrderPropertyResponse { property } = send_assert_json(
            &client,
            client
                .get(url.join("/documents/d1/properties/order")?)
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;
        assert_eq!(property, 3);
        let OrderPropertyResponse { property } = send_assert_json(
            &client,
            client
                .get(url.join("/documents/d2/properties/order")?)
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;
        assert_eq!(property, 2);
        Ok(())
    });
}

#[test]
fn test_ingestion_validation() {
    test_app::<WebApi, _>(
        Some(toml! {
            [ingestion]
            max_snippet_size = 10
            max_properties_size = 10
        }),
        |client, url, _| async move {
            let error = send_assert_json::<Error>(
                &client,
                client
                    .post(url.join("/documents")?)
                    .json(&json!({
                        "documents": [
                            { "id": "d1", "snippet": "abc\x00" },
                        ]
                    }))
                    .build()?,
                StatusCode::BAD_REQUEST,
                false,
            )
            .await;
            assert_eq!(error.kind, Kind::FailedToValidateDocuments);
            assert_eq!(
                error.details.unwrap(),
                Details::Ingest(vec![json!({
                    "id": "d1",
                    "kind": "InvalidDocumentSnippet",
                    "details": {
                        "invalid_string": {
                            "syntax": {
                                "expected": "^[^\\x00]*$"
                            }
                        }
                    }
                })]),
            );

            let error = send_assert_json::<Error>(
                &client,
                client
                    .post(url.join("/documents")?)
                    .json(&json!({
                        "documents": [
                            { "id": "d1", "snippet": "abcdefghijk" },
                        ]
                    }))
                    .build()?,
                StatusCode::BAD_REQUEST,
                false,
            )
            .await;
            assert_eq!(error.kind, Kind::FailedToValidateDocuments);
            assert_eq!(
                error.details.unwrap(),
                Details::Ingest(vec![json!({
                    "id": "d1",
                    "kind": "InvalidDocumentSnippet",
                    "details": {
                        "invalid_string": {
                            "size": {
                                "got": 11,
                                "bounds": "1..=10",
                            }
                        }
                    }
                })]),
            );

            let error = send_assert_json::<Error>(
                &client,
                client
                    .post(url.join("/documents")?)
                    .json(&json!({
                        "documents": [
                            { "id": "d1", "snippet": "abc", "properties": { "p": "defghijklm" } },
                        ]
                    }))
                    .build()?,
                StatusCode::BAD_REQUEST,
                false,
            )
            .await;
            assert_eq!(error.kind, Kind::FailedToValidateDocuments);
            assert_eq!(
                error.details.unwrap(),
                Details::Ingest(vec![json!({
                    "id": "d1",
                    "kind": "InvalidDocumentProperties",
                    "details": {
                        "storage_size": {
                            "got": 27,
                            "max": 10,
                        },
                    }
                })]),
            );

            Ok(())
        },
    );
}
