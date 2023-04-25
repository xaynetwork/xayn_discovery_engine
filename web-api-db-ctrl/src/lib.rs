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

mod postgres;

use sqlx::pool::PoolOptions;
use xayn_web_api_shared::{
    postgres::{Client as PgClient, Config as PgConfig},
    request::TenantId,
};

//TODO
pub type Error = anyhow::Error;

pub struct Config {
    pub postgres: PgConfig,
    pub enable_legacy_tenant: bool,
}

#[derive(Clone)]
pub struct Silo {
    postgres: PgClient,
    enable_legacy_tenant: bool,
}

impl Silo {
    pub async fn builder(config: Config) -> Result<Self, Error> {
        let postgres = PoolOptions::new()
            .connect_with(config.postgres.to_connection_options()?)
            .await?;

        Ok(Self {
            postgres,
            enable_legacy_tenant: config.enable_legacy_tenant,
        })
    }

    pub async fn initialize(&self) -> Result<Option<TenantId>, Error> {
        postgres::initialize(&self.postgres, self.enable_legacy_tenant).await
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
        //TODO elastic::create_tenant(&self.elastic, new_id).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_tenant(&self, tenant_id: &TenantId) -> Result<(), Error> {
        let mut tx = self.postgres.begin().await?;
        postgres::delete_tenant(&mut tx, tenant_id).await?;
        //TODO elastic::delete_tenant(&self.elastic, tenant_id)
        tx.commit().await?;
        Ok(())
    }
}
