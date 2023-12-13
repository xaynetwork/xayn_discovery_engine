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

use anyhow::{anyhow, Error};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use xayn_web_api_shared::request::TenantId;

//Hint: Silo API stability: This is currently directly serialized and returned from the /silo_management API.
//      If we do any braking changes wrt. serialization format we need to create a serde proxy struct.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "TenantSerdeProxy")]
pub struct Tenant {
    pub tenant_id: TenantId,
    pub is_legacy_tenant: bool,
    pub es_index_name: String,
}

#[derive(Clone, Debug, Deserialize)]
struct TenantSerdeProxy {
    tenant_id: TenantId,
    is_legacy_tenant: bool,
    #[serde(default)]
    es_index_name: Option<String>,
}

impl From<TenantSerdeProxy> for Tenant {
    fn from(
        TenantSerdeProxy {
            tenant_id,
            is_legacy_tenant,
            es_index_name,
        }: TenantSerdeProxy,
    ) -> Self {
        Self::new_with_defaults(tenant_id, is_legacy_tenant, es_index_name)
    }
}

impl Tenant {
    pub async fn load_from_postgres(
        connection: &mut PgConnection,
        tenant_id: TenantId,
    ) -> Result<Tenant, Error> {
        let (is_legacy_tenant, es_index_name) = sqlx::query_as::<_, (bool, Option<String>)>(
            "SELECT is_legacy_tenant, es_index_name
            FROM management.tenant
            WHERE tenant_id = $1;",
        )
        .bind(&tenant_id)
        .fetch_optional(connection)
        .await?
        .ok_or_else(|| anyhow!("unknown tenant: {tenant_id}"))?;

        Ok(Tenant::new_with_defaults(
            tenant_id,
            is_legacy_tenant,
            es_index_name,
        ))
    }

    pub fn new_with_defaults(
        tenant_id: TenantId,
        is_legacy_tenant: bool,
        es_index_name: Option<String>,
    ) -> Self {
        let es_index_name = es_index_name.unwrap_or_else(|| tenant_id.to_string());
        Self {
            tenant_id,
            is_legacy_tenant,
            es_index_name,
        }
    }
}
