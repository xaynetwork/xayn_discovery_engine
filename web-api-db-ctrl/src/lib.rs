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

mod elastic;
mod postgres;

use sqlx::pool::PoolOptions;
use xayn_web_api_shared::{
    elastic::{Client as EsClient, Config as EsConfig},
    postgres::{Client as PgClient, Config as PgConfig},
    request::TenantId,
};

//TODO
pub type Error = anyhow::Error;

#[derive(Clone)]
pub struct Silo {
    postgres: PgClient,
    elastic: EsClient,
    enable_legacy_tenant: Option<LegacyTenantInfo>,
}

#[derive(Clone)]
pub struct LegacyTenantInfo {
    pub es_index: String,
}

impl Silo {
    pub async fn new(
        postgres: &PgConfig,
        elastic: EsConfig,
        enable_legacy_tenant: Option<LegacyTenantInfo>,
    ) -> Result<Self, Error> {
        let postgres = PoolOptions::new()
            .connect_with(postgres.to_connection_options()?)
            .await?;

        let elastic = EsClient::new(elastic)?;

        Ok(Self {
            postgres,
            elastic,
            enable_legacy_tenant,
        })
    }

    pub async fn initialize(&self) -> Result<Option<TenantId>, Error> {
        let opt_legacy_setup = self.enable_legacy_tenant.as_ref().map(move |legacy_info| {
            move |tenant_id| async move {
                elastic::setup_legacy_tenant(&self.elastic, &legacy_info.es_index, &tenant_id).await
            }
        });
        postgres::initialize(&self.postgres, opt_legacy_setup).await
    }

    pub async fn admin_as_mt_user_hack(&self) -> Result<(), Error> {
        postgres::admin_as_mt_user_hack(&self.postgres).await
    }

    pub async fn list_tenants(&self) -> Result<Vec<TenantId>, Error> {
        postgres::list_tenants(&self.postgres).await
    }

    pub async fn create_tenant(&self, new_id: &TenantId) -> Result<(), Error> {
        let mut tx = self.postgres.begin().await?;
        postgres::create_tenant(&mut tx, new_id).await?;
        elastic::create_tenant(&self.elastic, new_id).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_tenant(&self, tenant_id: &TenantId) -> Result<(), Error> {
        let mut tx = self.postgres.begin().await?;
        postgres::delete_tenant(&mut tx, tenant_id).await?;
        elastic::delete_tenant(&self.elastic, tenant_id).await?;
        tx.commit().await?;
        Ok(())
    }
}
