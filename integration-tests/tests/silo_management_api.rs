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

use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use toml::toml;
use xayn_integration_tests::{send_assert_json, test_app};
use xayn_web_api::WebApi;
use xayn_web_api_db_ctrl::{tenant::Tenant, OperationResult};
use xayn_web_api_shared::request::TenantId;

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
                    tenant: Tenant::new_with_defaults(make_id("1")?, false, None),
                }
            );
            assert_eq!(
                results.next().unwrap(),
                OperationResult::CreateTenant {
                    tenant: Tenant::new_with_defaults(make_id("3")?, true, None)
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
                    Tenant::new_with_defaults(make_id("1")?, false, None),
                    Tenant::new_with_defaults(make_id("3")?, true, None),
                ]
                .iter()
                .collect::<HashSet<_>>()
            );
            assert_eq!(
                results.next().unwrap(),
                OperationResult::DeleteTenant {
                    tenant: Some(Tenant::new_with_defaults(make_id("3")?, true, None))
                }
            );
            let OperationResult::ListTenants { tenants } = results.next().unwrap() else {
                panic!("failed to list tenants");
            };
            assert_eq!(
                tenants.iter().collect::<HashSet<_>>(),
                [
                    services.tenant.clone(),
                    Tenant::new_with_defaults(make_id("1")?, false, None)
                ]
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
