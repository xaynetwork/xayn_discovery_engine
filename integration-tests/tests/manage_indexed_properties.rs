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

use reqwest::StatusCode;
use serde_json::{json, Value};
use toml::toml;
use url::Url;
use xayn_integration_tests::{send_assert, send_assert_json, test_app, UNCHANGED_CONFIG};
use xayn_web_api::WebApi;

#[test]
fn test_creating_indexed_properties() {
    test_app::<WebApi, _>(UNCHANGED_CONFIG, |client, url, services| async move {
        let res = send_assert_json::<Value>(
            &client,
            client
                .post(url.join("/documents/_indexed_properties")?)
                .json(&json!({
                    "properties": {
                        "p1": {
                            "type": "boolean"
                        },
                        "p2": {
                            "type": "number"
                        },
                        "p3": {
                            "type": "keyword"
                        },
                        "p4": {
                            "type": "keyword[]"
                        },
                        "p5": {
                            "type": "date"
                        }
                    }
                }))
                .build()?,
            StatusCode::ACCEPTED,
            false,
        )
        .await;

        assert_eq!(
            res,
            json!({
                "properties": {
                    "p1": {
                        "type": "boolean"
                    },
                    "p2": {
                        "type": "number"
                    },
                    "p3": {
                        "type": "keyword"
                    },
                    "p4": {
                        "type": "keyword[]"
                    },
                    "p5": {
                        "type": "date"
                    },
                    "publication_date": {
                        "type": "date"
                    }
                }
            })
        );

        let res = send_assert_json::<Value>(
            &client,
            client
                .post(url.join("/documents/_indexed_properties")?)
                .json(&json!({
                    "properties": {
                        "p6": {
                            "type": "keyword"
                        }
                    }
                }))
                .build()?,
            StatusCode::ACCEPTED,
            false,
        )
        .await;

        assert_eq!(
            res,
            json!({
                "properties": {
                    "p1": {
                        "type": "boolean"
                    },
                    "p2": {
                        "type": "number"
                    },
                    "p3": {
                        "type": "keyword"
                    },
                    "p4": {
                        "type": "keyword[]"
                    },
                    "p5": {
                        "type": "date"
                    },
                    "p6": {
                        "type": "keyword"
                    },
                    "publication_date": {
                        "type": "date"
                    }
                }
            })
        );

        let res2 = send_assert_json::<Value>(
            &client,
            client
                .get(url.join("/documents/_indexed_properties")?)
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;

        assert_eq!(res, res2);

        let es = services.silo.elastic_config();
        let url = es.url.parse::<Url>()?.join("_mapping")?;
        let res =
            send_assert_json::<Value>(&client, client.get(url).build()?, StatusCode::OK, false)
                .await;
        let properties_mapping =
            &res[services.test_id.as_str()]["mappings"]["properties"]["properties"]["properties"];
        assert_eq!(
            properties_mapping,
            &json!({
                "publication_date": {
                    "type": "date",
                    "ignore_malformed": true
                },
                "p1": {
                    "type": "boolean",
                    "ignore_malformed": true,
                },
                "p2": {
                    "type": "double",
                    "ignore_malformed": true,
                },
                "p3": {
                    "type": "keyword"
                },
                "p4": {
                    "type": "keyword"
                },
                "p5": {
                    "type": "date",
                    "ignore_malformed": true,
                },
                "p6": {
                    "type": "keyword"
                }
            })
        );

        Ok(())
    });
}

#[test]
fn test_check_new_property_values_against_schema() {
    test_app::<WebApi, _>(
        Some(toml! {
            [ingestion.index_update]
            method = "background"
        }),
        |client, url, _| async move {
            let mut count = 0;
            let mut make_id = || {
                count += 1;
                format!("d{}", count)
            };
            send_assert(
            &client,
            client
                .post(url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": make_id(), "snippet": "snippet 1", "properties": { "p2": "bad" } },
                        { "id": make_id(), "snippet": "snippet 2", "properties": { "p2": 12 } },
                    ]
                }))
                .build()?,
            StatusCode::CREATED,
            false,
        )
        .await;

            let res = send_assert_json::<Value>(
                &client,
                client
                    .post(url.join("/documents/_indexed_properties")?)
                    .json(&json!({
                        "properties": {
                            "p1": {
                                "type": "boolean"
                            },
                            "p2": {
                                "type": "number"
                            },
                            "p3": {
                                "type": "keyword"
                            },
                            "p4": {
                                "type": "keyword[]"
                            },
                            "p5": {
                                "type": "date"
                            }
                        }
                    }))
                    .build()?,
                StatusCode::ACCEPTED,
                false,
            )
            .await;

            assert_eq!(
                res,
                json!({
                    "properties": {
                        "p1": {
                            "type": "boolean"
                        },
                        "p2": {
                            "type": "number"
                        },
                        "p3": {
                            "type": "keyword"
                        },
                        "p4": {
                            "type": "keyword[]"
                        },
                        "p5": {
                            "type": "date"
                        },
                        "publication_date": {
                            "type": "date"
                        }
                    }
                })
            );

            send_assert(
                &client,
                client
                    .post(url.join("/documents")?)
                    .json(&json!({
                        "documents": [
                            { "id": make_id(), "snippet": "snippet 3", "properties": {
                                "p1": true,
                                "p2": 21,
                                "p3": "a label",
                                "p4": ["one", "two"],
                                "p5": "2023-06-26T07:23:54Z"
                            } }
                        ]
                    }))
                    .build()?,
                StatusCode::CREATED,
                false,
            )
            .await;

            for (property, bad_value, expected_type) in [
                ("p1", Value::String("bad".into()), "boolean"),
                ("p2", "bad".into(), "number"),
                ("p3", 123.into(), "keyword"),
                ("p4", 32.into(), "keyword[]"),
                ("p4", "bad".into(), "keyword[]"),
                ("p5", false.into(), "date"),
            ] {
                let id = make_id();
                let res = send_assert_json::<Value>(
                    &client,
                    client
                        .post(url.join("/documents")?)
                        .json(&json!({
                            "documents": [
                                { "id": id, "snippet": "snippet 3", "properties": {
                                    property: bad_value
                                } }
                            ]
                        }))
                        .build()?,
                    StatusCode::BAD_REQUEST,
                    false,
                )
                .await;

                assert_eq!(
                    &res["details"]["documents"],
                    &json!([{
                        "id": id,
                        "kind": "InvalidDocumentProperty",
                        "details": {
                            "invalid_reason": {
                                "IncompatibleType": {
                                    "expected": expected_type,
                                }
                            },
                            "invalid_value": bad_value,
                            "property_id": property,
                        }
                    }])
                );
            }

            for (property, bad_value, expected_type) in [
                ("p1", Value::String("bad".into()), "boolean"),
                ("p2", "bad".into(), "number"),
                ("p3", 123.into(), "keyword"),
                ("p4", 32.into(), "keyword[]"),
                ("p4", "bad".into(), "keyword[]"),
                ("p5", false.into(), "date"),
            ] {
                let res = send_assert_json::<Value>(
                    &client,
                    client
                        .put(url.join("/documents/d1/properties")?)
                        .json(&json!({
                            "properties": {
                                property: bad_value,
                            },
                        }))
                        .build()?,
                    StatusCode::BAD_REQUEST,
                    false,
                )
                .await;

                assert_eq!(&res["kind"], &json!("InvalidDocumentProperty"));
                assert_eq!(
                    &res["details"],
                    &json!({
                        "property_id": property,
                        "invalid_reason": {
                            "IncompatibleType": {
                                "expected": expected_type
                            },
                        },
                        "invalid_value": bad_value,
                    })
                );
            }

            for (property, bad_value, expected_type) in [
                ("p1", Value::String("bad".into()), "boolean"),
                ("p2", "bad".into(), "number"),
                ("p3", 123.into(), "keyword"),
                ("p4", 32.into(), "keyword[]"),
                ("p4", "bad".into(), "keyword[]"),
                ("p5", false.into(), "date"),
            ] {
                let mut url = Url::clone(&url);
                url.path_segments_mut().unwrap().extend([
                    "documents",
                    "d2",
                    "properties",
                    property,
                ]);

                let res = send_assert_json::<Value>(
                    &client,
                    client
                        .put(url)
                        .json(&json!({ "property": bad_value }))
                        .build()?,
                    StatusCode::BAD_REQUEST,
                    false,
                )
                .await;

                assert_eq!(&res["kind"], &json!("InvalidDocumentProperty"));
                assert_eq!(
                    &res["details"],
                    &json!({
                        "property_id": property,
                        "invalid_reason": {
                            "IncompatibleType": {
                                "expected": expected_type
                            },
                        },
                        "invalid_value": bad_value,
                    })
                );
            }

            Ok(())
        },
    );
}
