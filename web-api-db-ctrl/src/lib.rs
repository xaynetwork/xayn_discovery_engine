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

pub use elastic::create_tenant_index as elastic_create_tenant;
use serde::{Deserialize, Serialize};
use sqlx::pool::PoolOptions;
use xayn_web_api_shared::{
    elastic::{Client as EsClient, Config as EsConfig},
    pinecone::{Client as PcClient, Config as PcConfig},
    postgres::{Client as PgClient, Config as PgConfig},
    request::TenantId,
};

//TODO
pub type Error = anyhow::Error;

#[derive(Clone, Debug)]
pub struct Silo {
    postgres_config: PgConfig,
    elastic_config: EsConfig,
    pinecone_config: PcConfig,
    postgres: PgClient,
    elastic: EsClient,
    #[allow(dead_code)] // TODO: multi-tenancy and legacy setup
    pinecone: PcClient,
    enable_legacy_tenant: Option<LegacyTenantInfo>,
    embedding_size: usize,
}

#[derive(Clone, Debug)]
pub struct LegacyTenantInfo {
    pub es_index: String,
}

impl Silo {
    pub async fn new(
        postgres_config: PgConfig,
        elastic_config: EsConfig,
        pinecone_config: PcConfig,
        enable_legacy_tenant: Option<LegacyTenantInfo>,
        embedding_size: usize,
    ) -> Result<Self, Error> {
        let postgres = PoolOptions::new()
            .connect_with(postgres_config.to_connection_options()?)
            .await?;

        let elastic = EsClient::new(elastic_config.clone())?;
        let pinecone = PcClient::new(pinecone_config.clone())?;

        Ok(Self {
            postgres_config,
            elastic_config,
            pinecone_config,
            postgres,
            elastic,
            pinecone,
            enable_legacy_tenant,
            embedding_size,
        })
    }

    pub async fn initialize(&self) -> Result<Option<TenantId>, Error> {
        let opt_legacy_setup = self.enable_legacy_tenant.as_ref().map(move |legacy_info| {
            move |tenant_id| async move {
                elastic::setup_legacy_tenant(
                    &self.elastic,
                    &legacy_info.es_index,
                    &tenant_id,
                    self.embedding_size,
                )
                .await
            }
        });
        let migrate_tenant = move |tenant_id| async move {
            elastic::migrate_tenant_index(&self.elastic, &tenant_id, self.embedding_size).await
        };

        postgres::initialize(&self.postgres, opt_legacy_setup, migrate_tenant).await
    }

    pub async fn admin_as_mt_user_hack(&self) -> Result<(), Error> {
        postgres::admin_as_mt_user_hack(&self.postgres).await
    }

    pub async fn list_tenants(&self) -> Result<Vec<Tenant>, Error> {
        postgres::list_tenants(&self.postgres).await
    }

    pub async fn create_tenant(
        &self,
        tenant_id: TenantId,
        is_legacy_tenant: bool,
    ) -> Result<Tenant, Error> {
        let mut tx = self.postgres.begin().await?;
        postgres::create_tenant(&mut tx, &tenant_id, is_legacy_tenant).await?;
        elastic::create_tenant_index(&self.elastic, &tenant_id, self.embedding_size).await?;
        tx.commit().await?;
        Ok(Tenant {
            tenant_id,
            is_legacy_tenant,
        })
    }

    pub async fn delete_tenant(&self, tenant_id: TenantId) -> Result<Option<Tenant>, Error> {
        let mut tx = self.postgres.begin().await?;
        let deleted_tenant = postgres::delete_tenant(&mut tx, tenant_id).await?;
        if let Some(tenant) = &deleted_tenant {
            elastic::delete_tenant(&self.elastic, &tenant.tenant_id).await?;
        }
        tx.commit().await?;
        Ok(deleted_tenant)
    }

    pub async fn run_operations(
        &self,
        initialize: bool,
        ops: impl IntoIterator<Item = Operation>,
    ) -> Result<Vec<OperationResult>, Error> {
        if initialize {
            self.initialize().await?;
        }

        let mut results = Vec::new();
        for op in ops {
            results.push(self.run_operation(op).await);
        }
        Ok(results)
    }

    async fn run_operation(&self, op: Operation) -> OperationResult {
        match op {
            Operation::ListTenants {} => self
                .list_tenants()
                .await
                .map(|tenants| OperationResult::ListTenants { tenants })
                .unwrap_or_else(|err| OperationResult::Error {
                    msg: err.to_string(),
                }),
            Operation::CreateTenant {
                tenant_id,
                is_legacy_tenant,
            } => self
                .create_tenant(tenant_id, is_legacy_tenant)
                .await
                .map(|tenant| OperationResult::CreateTenant { tenant })
                .unwrap_or_else(|err| OperationResult::Error {
                    msg: err.to_string(),
                }),
            Operation::DeleteTenant { tenant_id } => self
                .delete_tenant(tenant_id)
                .await
                .map(|tenant| OperationResult::DeleteTenant { tenant })
                .unwrap_or_else(|err| OperationResult::Error {
                    msg: err.to_string(),
                }),
        }
    }

    pub fn postgres_config(&self) -> &PgConfig {
        &self.postgres_config
    }

    pub fn elastic_config(&self) -> &EsConfig {
        &self.elastic_config
    }

    pub fn pinecone_config(&self) -> &PcConfig {
        &self.pinecone_config
    }
}

#[derive(Deserialize, Debug)]
pub enum Operation {
    ListTenants {},
    CreateTenant {
        tenant_id: TenantId,
        #[serde(default)]
        is_legacy_tenant: bool,
    },
    DeleteTenant {
        tenant_id: TenantId,
    },
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum OperationResult {
    ListTenants { tenants: Vec<Tenant> },
    CreateTenant { tenant: Tenant },
    DeleteTenant { tenant: Option<Tenant> },
    Error { msg: String },
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tenant {
    pub tenant_id: TenantId,
    pub is_legacy_tenant: bool,
}
