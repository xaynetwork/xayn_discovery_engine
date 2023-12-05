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

use sqlx::PgConnection;
use xayn_web_api_shared::request::TenantId;

use crate::{error::common::InternalError, Error};

pub(crate) struct TenantConfig {
    pub(crate) tenant_id: TenantId,
    #[allow(dead_code)]
    pub(crate) is_legacy_tenant: bool,
    pub(crate) es_index_name: String,
}

impl TenantConfig {
    pub(super) async fn load_from_postgres(
        connection: &mut PgConnection,
        tenant_id: TenantId,
    ) -> Result<TenantConfig, Error> {
        let (is_legacy_tenant, es_index_name) = sqlx::query_as::<_, (bool, Option<String>)>(
            "SELECT is_legacy_tenant, es_index_name
            FROM management.tenant
            WHERE tenant_id = $1;",
        )
        .bind(&tenant_id)
        .fetch_optional(connection)
        .await?
        .ok_or_else(|| InternalError::from_message(format!("unknown tenant: {tenant_id}")))?;

        let es_index_name = es_index_name.unwrap_or_else(|| tenant_id.to_string());
        Ok(Self {
            tenant_id,
            is_legacy_tenant,
            es_index_name,
        })
    }
}
