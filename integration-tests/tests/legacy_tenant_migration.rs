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

use anyhow::Error;
use chrono::{DateTime, TimeZone, Utc};
use sqlx::{Connection, PgConnection};
use tracing::instrument;
use xayn_integration_tests::{
    create_db,
    db_configs_for_testing,
    run_async_test,
    start_test_service_containers,
    TestId,
    MANAGEMENT_DB,
    TEST_EMBEDDING_SIZE,
};
use xayn_test_utils::env::clear_env;
use xayn_web_api_db_ctrl::{
    elastic_create_tenant,
    tenant::TenantWithOptionals,
    LegacyTenantInfo,
    Silo,
};
use xayn_web_api_shared::{
    elastic,
    postgres::{self, QuotedIdentifier},
    request::TenantId,
};

#[instrument(skip_all)]
async fn legacy_test_setup(test_id: &TestId) -> Result<(postgres::Config, elastic::Config), Error> {
    clear_env();
    start_test_service_containers();

    let (pg_config, es_config) = db_configs_for_testing(test_id);

    create_db(&pg_config, MANAGEMENT_DB).await?;

    Ok((pg_config, es_config))
}

#[test]
fn test_if_the_initializations_work_correctly_for_legacy_tenants() {
    run_async_test(|test_id| async move {
        let (pg_config, es_config) = legacy_test_setup(&test_id).await?;

        let pg_options = pg_config.to_connection_options()?;
        let mut conn = PgConnection::connect_with(&pg_options).await?;
        let mut tx = conn.begin().await?;

        sqlx::migrate!("../web-api-db-ctrl/postgres/tenant")
            .run(&mut tx)
            .await?;

        let es_client = elastic::ClientWithoutIndex::new(es_config.clone())?;
        let legacy_elastic_index_as_tenant_id =
            TenantId::try_parse_ascii(es_config.index_name.as_bytes())?;
        elastic_create_tenant(
            &es_client,
            &TenantWithOptionals {
                tenant_id: legacy_elastic_index_as_tenant_id,
                is_legacy_tenant: true,
                es_index_name: Some(es_config.index_name.clone()),
                model: None,
            }
            .into(),
            TEST_EMBEDDING_SIZE,
        )
        .await?;

        let user_id = "foo_boar";
        let last_seen = Utc.with_ymd_and_hms(2023, 2, 2, 3, 3, 3).unwrap();
        sqlx::query("INSERT INTO users(user_id, last_seen) VALUES ($1, $2)")
            .bind(user_id)
            .bind(last_seen)
            .execute(&mut tx)
            .await?;

        tx.commit().await?;
        conn.close().await?;

        let default_es_index = es_config.index_name.clone();
        let silo = Silo::new(
            pg_config,
            es_config,
            Some(LegacyTenantInfo {
                es_index: default_es_index,
            }),
            [("default".to_owned(), TEST_EMBEDDING_SIZE)].into(),
        )
        .await?;
        silo.admin_as_mt_user_hack().await?;
        let Some(legacy_tenant_id) = silo.initialize().await? else {
            panic!("initialization with legacy tenant didn't return a legacy tenant id");
        };
        let tenant_schema = QuotedIdentifier::db_name_for_tenant_id(&legacy_tenant_id);

        let mut conn = PgConnection::connect_with(&pg_options).await?;
        let query = format!("SELECT user_id, last_seen FROM {tenant_schema}.users;");
        let (found_user_id, found_last_seen) = sqlx::query_as::<_, (String, DateTime<Utc>)>(&query)
            .fetch_one(&mut conn)
            .await?;

        assert_eq!(found_user_id, user_id);
        assert_eq!(found_last_seen, last_seen);
        conn.close().await?;
        Ok(())
    })
}

#[test]
fn test_if_the_initializations_work_correctly_for_not_setup_legacy_tenants() {
    run_async_test(|test_id| async move {
        let (pg_config, es_config) = legacy_test_setup(&test_id).await?;

        let pg_options = pg_config.to_connection_options()?;

        let default_es_index = es_config.index_name.clone();
        let silo = Silo::new(
            pg_config,
            es_config,
            Some(LegacyTenantInfo {
                es_index: default_es_index,
            }),
            [("default".to_owned(), TEST_EMBEDDING_SIZE)].into(),
        )
        .await?;
        silo.admin_as_mt_user_hack().await?;
        let Some(legacy_tenant_id) = silo.initialize().await? else {
            panic!("initialization with legacy tenant didn't return a legacy tenant id");
        };
        let tenant_schema = QuotedIdentifier::db_name_for_tenant_id(&legacy_tenant_id);

        let mut conn = PgConnection::connect_with(&pg_options).await?;
        let query = format!("SELECT count(*) FROM {tenant_schema}.users;");
        let (count,) = sqlx::query_as::<_, (i64,)>(&query)
            .fetch_one(&mut conn)
            .await?;

        assert_eq!(count, 0);

        conn.close().await?;
        Ok(())
    })
}
