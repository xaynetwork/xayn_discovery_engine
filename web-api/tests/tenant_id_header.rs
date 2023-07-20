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

use reqwest::{Client, StatusCode};
use serde_json::json;
use toml::toml;
use xayn_integration_tests::{send_assert, test_app};
use xayn_web_api::Ingestion;

#[test]
fn test_tenant_id_is_required_if_legacy_tenant_is_disabled() {
    test_app::<Ingestion, _>(
        Some(toml! {
            [tenants]
            enable_legacy_tenant = false
        }),
        |_, url, _| async move {
            // don't use injected "X-Xayn-Tenant-Id" header
            let client = Client::new();
            send_assert(
                &client,
                client
                    .post(url.join("/documents")?)
                    .json(&json!({
                        "documents": [
                            { "id": "d1", "snippet": "once in a spring there was a fall" }
                        ]
                    }))
                    .build()?,
                StatusCode::INTERNAL_SERVER_ERROR,
                false,
            )
            .await;
            Ok(())
        },
    );
}

#[test]
fn test_tenant_id_is_not_required_if_legacy_tenant_is_enabled() {
    test_app::<Ingestion, _>(
        Some(toml! {
            [tenants]
            enable_legacy_tenant = true
        }),
        |_, url, _| async move {
            // don't use injected "X-Xayn-Tenant-Id" header
            let client = Client::new();
            send_assert(
                &client,
                client
                    .post(url.join("/documents")?)
                    .json(&json!({
                        "documents": [
                            { "id": "d1", "snippet": "once in a spring there was a fall" }
                        ]
                    }))
                    .build()?,
                StatusCode::CREATED,
                false,
            )
            .await;

            Ok(())
        },
    );
}
