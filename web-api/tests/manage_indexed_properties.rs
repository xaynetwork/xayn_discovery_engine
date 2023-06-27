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
use url::Url;
use xayn_integration_tests::{send_assert_json, test_app, UNCHANGED_CONFIG};
use xayn_web_api::Ingestion;

#[test]
fn test_creating_indexed_properties() {
    test_app::<Ingestion, _>(
        UNCHANGED_CONFIG,
        |client, ingestion_url, services| async move {
            let res = send_assert_json::<Value>(
                &client,
                client
                    .post(ingestion_url.join("/documents/_indexed_properties")?)
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
                    .post(ingestion_url.join("/documents/_indexed_properties")?)
                    .json(&json!({
                        "properties": {
                            "p6": {
                                "type": "keyword"
                            }
                        }
                    }))
                    .build()?,
                StatusCode::ACCEPTED,
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
                    .get(ingestion_url.join("/documents/_indexed_properties")?)
                    .build()?,
                StatusCode::OK,
            )
            .await;

            assert_eq!(res, res2);

            let es = services.silo.elastic_config();
            let url = es.url.parse::<Url>()?.join("_mapping")?;
            let res =
                send_assert_json::<Value>(&client, client.get(url).build()?, StatusCode::OK).await;
            let properties_mapping = &res[services.test_id.as_str()]["mappings"]["properties"]
                ["properties"]["properties"];
            assert_eq!(
                properties_mapping,
                &json!({
                    "publication_date": {
                        "type": "date"
                    },
                    "p1": {
                        "type": "boolean"
                    },
                    "p2": {
                        "type": "double"
                    },
                    "p3": {
                        "type": "keyword"
                    },
                    "p4": {
                        "type": "keyword"
                    },
                    "p5": {
                        "type": "date"
                    },
                    "p6": {
                        "type": "keyword"
                    }
                })
            );

            Ok(())
        },
    );
}
