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
use serde_json::{json, Value};
use url::Url;
use xayn_integration_tests::{
    send_assert,
    send_assert_json,
    test_two_apps,
    Services,
    UNCHANGED_CONFIG,
};
use xayn_web_api::{Ingestion, Personalization};
use xayn_web_api_shared::json_object;

async fn get_candidates(client: &Client, url: &Url) -> Result<HashSet<String>, Error> {
    let request = client.get(url.join("/documents/_candidates")?).build()?;
    let response: Response = send_assert_json(client, request, StatusCode::OK, false).await;
    return Ok(response.documents.into_iter().collect());

    #[derive(Deserialize)]
    struct Response {
        documents: Vec<String>,
    }
}

async fn set_candidates(
    client: &Client,
    url: &Url,
    ids: impl IntoIterator<Item = &str>,
) -> Result<(), Error> {
    let request = client
        .put(url.join("/documents/_candidates")?)
        .json(&json!({ "documents": ids.into_iter().map(|id| json!({ "id": id })).collect_vec() }))
        .build()?;
    send_assert(client, request, StatusCode::NO_CONTENT, false).await;

    Ok(())
}

async fn ingest(
    client: &Client,
    url: &Url,
    documents: impl IntoIterator<Item = (&str, &str)>,
) -> Result<(), Error> {
    let documents = documents
        .into_iter()
        .map(|(id, snippet)| json!({ "id": id, "snippet": snippet}))
        .collect_vec();

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

async fn delete(
    client: &Client,
    url: &Url,
    ids: impl IntoIterator<Item = &str>,
) -> Result<(), Error> {
    let request = client
        .delete(url.join("/documents")?)
        .json(&json!({ "documents": ids.into_iter().map(String::from).collect_vec() }))
        .build()?;
    send_assert(client, request, StatusCode::NO_CONTENT, false).await;
    Ok(())
}

async fn set_properties(
    client: &Client,
    url: &Url,
    id: &str,
    properties: &Value,
) -> Result<(), Error> {
    let mut url = url.clone();
    url.path_segments_mut()
        .unwrap()
        .extend(["documents", id, "properties"]);
    let request = client
        .put(url)
        .json(&json!({ "properties": properties }))
        .build()?;

    send_assert(client, request, StatusCode::NO_CONTENT, false).await;
    Ok(())
}

async fn get_properties(client: &Client, url: &Url, id: &str) -> Result<Value, Error> {
    let mut url = url.clone();
    url.path_segments_mut()
        .unwrap()
        .extend(["documents", id, "properties"]);
    let request = client.get(url).build()?;

    let response: Response =
        send_assert_json::<Response>(client, request, StatusCode::OK, false).await;

    return Ok(response.properties);

    #[derive(Deserialize)]
    struct Response {
        properties: Value,
    }
}

async fn documents_from_es(services: &Services) -> Result<HashSet<String>, Error> {
    let es_client = services
        .silo
        .elastic_client()
        .with_index(&services.tenant.tenant_id);

    let documents = es_client
        .search_request(
            json_object!({
                "query": {
                    "match_all": {}
                }
            }),
            Result::<_, Error>::Ok,
        )
        .await?;

    Ok(documents.into_keys().collect())
}

async fn interact(
    client: &Client,
    url: &Url,
    user: &str,
    documents: impl IntoIterator<Item = &str>,
) -> Result<(), Error> {
    let mut url = url.clone();
    url.path_segments_mut()
        .unwrap()
        .extend(["users", user, "interactions"]);
    let request = client
        .patch(url)
        .json(&json!({ "documents": documents.into_iter().map(|id| json!({ "id": id })).collect_vec() }))
        .build()?;
    send_assert(client, request, StatusCode::NO_CONTENT, false).await;
    Ok(())
}

async fn personalized_documents(
    client: &Client,
    url: &Url,
    user: &str,
) -> Result<Vec<String>, Error> {
    let mut url = url.clone();
    url.path_segments_mut()
        .unwrap()
        .extend(["users", user, "personalized_documents"]);
    let request = client.post(url).build()?;
    let response: Response = send_assert_json(client, request, StatusCode::OK, false).await;
    return Ok(response.documents.into_iter().map(|doc| doc.id).collect());

    #[derive(Deserialize)]
    struct Response {
        documents: Vec<Document>,
    }

    #[derive(Deserialize)]
    struct Document {
        id: String,
    }
}

fn string_set(x: impl IntoIterator<Item = impl Into<String>>) -> HashSet<String> {
    x.into_iter().map(Into::into).collect()
}

#[test]
fn test_deletes_them_from_elastic_search() {
    test_two_apps::<Ingestion, Personalization, _>(
        UNCHANGED_CONFIG,
        UNCHANGED_CONFIG,
        |client, ingestion_url, personalization_url, services| async move {
            ingest(&client, &ingestion_url, [("d1", "foo")]).await?;
            set_candidates(&client, &ingestion_url, ["d1"]).await?;
            assert_eq!(
                get_candidates(&client, &ingestion_url).await?,
                string_set(["d1"])
            );
            ingest(
                &client,
                &ingestion_url,
                [
                    ("d1", "daa"),
                    ("d2", "foo"),
                    ("d3", "bar"),
                    ("d4", "dee"),
                    ("d5", "doo"),
                    ("d6", "doo"),
                    ("d7", "eoo"),
                    ("d8", "aoo"),
                    ("d9", "uee"),
                ],
            )
            .await?;

            assert_eq!(
                get_candidates(&client, &ingestion_url).await?,
                string_set(["d1", "d2", "d3", "d4", "d5", "d6", "d7", "d8", "d9"])
            );
            delete(&client, &ingestion_url, ["d1"]).await?;
            let properties = json!({ "foo": "bar" });
            set_properties(&client, &ingestion_url, "d2", &properties).await?;
            assert_eq!(
                get_properties(&client, &ingestion_url, "d2").await?,
                properties
            );
            interact(&client, &personalization_url, "u1", ["d3", "d4"]).await?;
            interact(&client, &personalization_url, "u1", ["d3"]).await?;
            interact(&client, &personalization_url, "u1", ["d7"]).await?;

            assert!(!personalized_documents(&client, &personalization_url, "u1")
                .await?
                .is_empty());

            let res: Value = send_assert_json(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "id": "d5" }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(res["documents"][0]["id"].as_str().unwrap(), "d6");

            let res: Value = send_assert_json(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "id": "d5" },
                        "personalize": {
                            "user": {
                                "history": [
                                    { "id": "d6" }
                                ]
                            }
                        }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert!(!res["documents"].as_array().unwrap().is_empty());

            let res: Value = send_assert_json(
                &client,
                client
                    .post(personalization_url.join("/semantic_search")?)
                    .json(&json!({
                        "document": { "query": "uee" }
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(res["documents"][0]["id"].as_str().unwrap(), "d9");

            set_candidates(&client, &ingestion_url, ["d5"]).await?;
            assert_eq!(documents_from_es(&services).await?, string_set(["d5"]));
            assert_eq!(
                personalized_documents(&client, &personalization_url, "u1").await?,
                vec!["d5".to_owned()]
            );

            delete(
                &client,
                &ingestion_url,
                ["d2", "d3", "d4", "d5", "d6", "d7", "d8", "d9"],
            )
            .await?;
            assert!(documents_from_es(&services).await?.is_empty());
            Ok(())
        },
    );
}
