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
#[serde(from = "TenantWithOptionals")]
pub struct Tenant {
    pub tenant_id: TenantId,
    pub is_legacy_tenant: bool,
    pub es_index_name: String,
    pub model: String,
}

/// A helper struct which allows specifying `None` for all fields which have default values.
///
/// It's also used as more stable API for `Deserializing` tenants.
#[derive(Clone, Debug, Deserialize)]
pub struct TenantWithOptionals {
    pub tenant_id: TenantId,
    #[serde(default)]
    pub is_legacy_tenant: bool,
    #[serde(default)]
    pub es_index_name: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

impl From<TenantWithOptionals> for Tenant {
    fn from(
        TenantWithOptionals {
            tenant_id,
            is_legacy_tenant,
            es_index_name,
            model,
        }: TenantWithOptionals,
    ) -> Self {
        let es_index_name = es_index_name.unwrap_or_else(|| tenant_id.to_string());
        let model = model.unwrap_or_else(|| "default".to_string());
        Self {
            tenant_id,
            is_legacy_tenant,
            es_index_name,
            model,
        }
    }
}

impl Tenant {
    pub async fn load_from_postgres(
        connection: &mut PgConnection,
        tenant_id: TenantId,
    ) -> Result<Tenant, Error> {
        let (is_legacy_tenant, es_index_name, model) =
            sqlx::query_as::<_, (bool, Option<String>, Option<String>)>(
                "SELECT is_legacy_tenant, es_index_name, model
                FROM management.tenant
                WHERE tenant_id = $1;",
            )
            .bind(&tenant_id)
            .fetch_optional(connection)
            .await?
            .ok_or_else(|| anyhow!("unknown tenant: {tenant_id}"))?;

        Ok(TenantWithOptionals {
            tenant_id,
            is_legacy_tenant,
            es_index_name,
            model,
        }
        .into())
    }
}
