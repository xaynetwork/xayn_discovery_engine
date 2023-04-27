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

use chrono::{DateTime, TimeZone, Utc};
use sqlx::{Connection, PgConnection};
use xayn_integration_tests::{crate_db, db_configs_for_testing, generate_test_id, MANAGEMENT_DB};
use xayn_web_api_db_ctrl::{LegacyTenantInfo, Silo};
use xayn_web_api_shared::postgres::QuotedIdentifier;

#[tokio::test]
async fn test_if_the_initializations_works_correct_for_legacy_tenants() -> Result<(), anyhow::Error>
{
    let test_id = generate_test_id();
    let (pg_config, es_config) = db_configs_for_testing(&test_id);

    crate_db(&pg_config, MANAGEMENT_DB).await?;

    let pg_options = pg_config.to_connection_options()?;
    let mut conn = PgConnection::connect_with(&pg_options).await?;

    sqlx::migrate!("../web-api-db-ctrl/postgres/tenant")
        .run(&mut conn)
        .await?;

    let user_id = "foo_boar";
    let last_seen = Utc.with_ymd_and_hms(2023, 2, 2, 3, 3, 3).unwrap();
    sqlx::query("INSERT INTO users(user_id, last_seen) VALUES ($1, $2)")
        .bind(user_id)
        .bind(last_seen)
        .execute(&mut conn)
        .await?;

    conn.close().await?;

    let default_es_index = es_config.index_name.clone();
    let silo = Silo::new(
        pg_config,
        es_config,
        Some(LegacyTenantInfo {
            es_index: default_es_index,
        }),
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
}
