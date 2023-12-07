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

use std::{collections::HashSet, thread};

use anyhow::Error;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::json;
use tokio::runtime::Runtime;
use toml::toml;
use url::Url;
use xayn_integration_tests::{send_assert, send_assert_json, test_app, TEST_EMBEDDING_SIZE};
use xayn_web_api::WebApi;
use xayn_web_api_db_ctrl::{elastic, tenant::TenantWithOptionals, OperationResult};
use xayn_web_api_shared::{
    elastic::ClientWithoutIndex,
    json_object,
    request::{InvalidTenantId, TenantId},
};

#[derive(Deserialize)]
struct ManagementResponse {
    results: Vec<OperationResult>,
}

#[test]
fn test_tenants_can_be_created() {
    test_app::<WebApi, _>(
        Some(toml! {
            [tenants]
            enable_legacy_tenant = false
        }),
        |client, url, services| async move {
            let test_id = &services.test_id;
            let make_id = |suffix| format!("{test_id}_{suffix}").parse::<TenantId>();
            let test_tenant = |suffix, is_legacy_tenant| {
                Result::<_, InvalidTenantId>::Ok(
                    TenantWithOptionals {
                        tenant_id: make_id(suffix)?,
                        is_legacy_tenant,
                        es_index_name: None,
                        model: None,
                    }
                    .into(),
                )
            };
            let ManagementResponse { results } = send_assert_json(
                &client,
                client.post(url.join("/_ops/silo_management")?).json(&json!({
                    "operations": [
                        { "CreateTenant": { "tenant_id": make_id("1")? }},
                        { "CreateTenant": { "tenant_id": make_id("3")?, "is_legacy_tenant": true }},
                        { "CreateTenant": { "tenant_id": make_id("1")? }},
                        { "ListTenants": {} },
                        { "DeleteTenant": { "tenant_id": make_id("3")? }},
                        { "ListTenants": {} },
                        { "DeleteTenant": { "tenant_id": make_id("3")? }},
                    ]
                })).build()?,
                StatusCode::OK,
                false,
            )
            .await;

            let mut results = results.into_iter();

            assert_eq!(
                results.next().unwrap(),
                OperationResult::CreateTenant {
                    tenant: test_tenant("1", false)?,
                }
            );
            assert_eq!(
                results.next().unwrap(),
                OperationResult::CreateTenant {
                    tenant: test_tenant("3", true)?,
                }
            );
            assert!(matches!(
                &results.next().unwrap(),
                OperationResult::Error { .. }
            ));
            let OperationResult::ListTenants { tenants } = results.next().unwrap() else {
                panic!("failed to list tenants");
            };
            assert_eq!(
                tenants.iter().collect::<HashSet<_>>(),
                [
                    services.tenant.clone(),
                    test_tenant("1", false)?,
                    test_tenant("3", true)?,
                ]
                .iter()
                .collect::<HashSet<_>>()
            );
            assert_eq!(
                results.next().unwrap(),
                OperationResult::DeleteTenant {
                    tenant: Some(test_tenant("3", true)?)
                }
            );
            let OperationResult::ListTenants { tenants } = results.next().unwrap() else {
                panic!("failed to list tenants");
            };
            assert_eq!(
                tenants.iter().collect::<HashSet<_>>(),
                [services.tenant.clone(), test_tenant("1", false)?,]
                    .iter()
                    .collect()
            );
            assert_eq!(
                results.next().unwrap(),
                OperationResult::DeleteTenant { tenant: None }
            );

            assert_eq!(results.next(), None);
            Ok(())
        },
    );
}

async fn ingest(client: &Client, url: &Url, documents: Vec<(&str, &str)>) -> Result<(), Error> {
    let documents = documents
        .into_iter()
        .map(|(id, snippet)| json_object!({ "id": id, "snippet": snippet }))
        .collect::<Vec<_>>();

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

async fn search(client: &Client, url: &Url) -> Result<HashSet<String>, Error> {
    #[derive(Deserialize)]
    struct Response {
        documents: Vec<Document>,
    }
    #[derive(Deserialize)]
    struct Document {
        id: String,
    }

    Ok(send_assert_json::<Response>(
        client,
        client
            .post(url.join("/semantic_search")?)
            .json(&json!({
                "document": { "query": "document" }
            }))
            .build()?,
        StatusCode::OK,
        false,
    )
    .await
    .documents
    .into_iter()
    .map(|d| d.id)
    .collect())
}

#[test]
fn test_changing_the_es_index_works() {
    const TEST_INDEX: &str = "test_changing_the_es_index_works";
    test_app::<WebApi, _>(
        Some(toml! {
            [tenants]
            enable_legacy_tenant = false
        }),
        |client, url, services| async move {
            struct CleanUp(Option<ClientWithoutIndex>);
            impl Drop for CleanUp {
                fn drop(&mut self) {
                    let client = self.0.take().unwrap();
                    // thread is needed as you can't create a `Runtime` in a thread currently executing a `Runtime`
                    thread::spawn(move || {
                        let rt = Runtime::new().unwrap();
                        rt.block_on(async { elastic::delete_index(&client, TEST_INDEX).await })
                            .unwrap();
                    });
                }
            }
            let _cleanup = CleanUp(Some(services.silo.elastic_client().clone()));

            ingest(&client, &url, vec![("d0", "document 0")]).await?;
            assert_eq!(search(&client, &url).await?, ["d0".to_owned()].into());

            let ManagementResponse { results } = send_assert_json(
                &client,
                client
                    .post(url.join("/_ops/silo_management")?)
                    .json(&json!({
                        "operations": [
                            { "ChangeEsIndex": {
                                "tenant_id": &services.tenant.tenant_id,
                                "es_index_name": TEST_INDEX
                             } },
                        ]
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;

            assert_eq!(
                results,
                vec![OperationResult::Error {
                    msg: "index the tenant is supposed to switch to doesn't exist".into()
                }]
            );

            elastic::create_tenant_index(
                services.silo.elastic_client(),
                &TenantWithOptionals {
                    tenant_id: services.tenant.tenant_id.clone(),
                    is_legacy_tenant: false,
                    es_index_name: Some(TEST_INDEX.to_owned()),
                    model: None,
                }
                .into(),
                TEST_EMBEDDING_SIZE,
            )
            .await?;

            let ManagementResponse { results } = send_assert_json(
                &client,
                client
                    .post(url.join("/_ops/silo_management")?)
                    .json(&json!({
                        "operations": [
                            { "ChangeEsIndex": {
                                "tenant_id": &services.tenant.tenant_id,
                                "es_index_name": TEST_INDEX,
                             } },
                        ]
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(results, vec![OperationResult::Success]);

            assert_eq!(search(&client, &url).await?, [].into());

            let ManagementResponse { results } = send_assert_json(
                &client,
                client
                    .post(url.join("/_ops/silo_management")?)
                    .json(&json!({
                        "operations": [
                            { "ChangeEsIndex": {
                                "tenant_id": &services.tenant.tenant_id,
                                "es_index_name": &services.tenant.tenant_id,
                             } },
                        ]
                    }))
                    .build()?,
                StatusCode::OK,
                false,
            )
            .await;
            assert_eq!(results, vec![OperationResult::Success]);

            assert_eq!(search(&client, &url).await?, ["d0".to_owned()].into());

            Ok(())
        },
    )
}
