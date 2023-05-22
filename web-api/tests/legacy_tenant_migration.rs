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

use std::{collections::HashSet, time::Duration};

use anyhow::Error;
use chrono::{DateTime, TimeZone, Utc};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Connection, Executor, PgConnection};
use toml::Table;
use tracing::instrument;
use xayn_integration_tests::{
    build_test_config_from_parts,
    create_db,
    db_configs_for_testing,
    generate_test_id,
    initialize_test_logging,
    start_test_service_containers,
    MANAGEMENT_DB,
};
use xayn_test_utils::env::clear_env;
use xayn_web_api::{config, start, Ingestion, Personalization};
use xayn_web_api_db_ctrl::{elastic_create_tenant, LegacyTenantInfo, Silo};
use xayn_web_api_shared::{
    elastic,
    postgres::{self, QuotedIdentifier},
    request::TenantId,
};

async fn legacy_test_setup() -> Result<(postgres::Config, elastic::Config), Error> {
    clear_env();
    initialize_test_logging();
    start_test_service_containers()?;

    let test_id = generate_test_id();
    let (pg_config, es_config) = db_configs_for_testing(&test_id);

    create_db(&pg_config, MANAGEMENT_DB).await?;

    Ok((pg_config, es_config))
}

#[tokio::test]
async fn test_if_the_initializations_work_correctly_for_legacy_tenants() -> Result<(), Error> {
    let (pg_config, es_config) = legacy_test_setup().await?;

    let pg_options = pg_config.to_connection_options()?;
    let mut conn = PgConnection::connect_with(&pg_options).await?;
    let mut tx = conn.begin().await?;

    sqlx::migrate!("../web-api-db-ctrl/postgres/tenant")
        .run(&mut tx)
        .await?;

    let es_client = elastic::Client::new(es_config.clone())?;
    let legacy_elastic_index_as_tenant_id =
        TenantId::try_parse_ascii(es_config.index_name.as_bytes())?;
    elastic_create_tenant(&es_client, &legacy_elastic_index_as_tenant_id).await?;

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

#[tokio::test]
async fn test_if_the_initializations_work_correctly_for_not_setup_legacy_tenants(
) -> Result<(), anyhow::Error> {
    let (pg_config, es_config) = legacy_test_setup().await?;

    let pg_options = pg_config.to_connection_options()?;

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
    let query = format!("SELECT count(*) FROM {tenant_schema}.users;");
    let (count,) = sqlx::query_as::<_, (i64,)>(&query)
        .fetch_one(&mut conn)
        .await?;

    assert_eq!(count, 0);

    conn.close().await?;
    Ok(())
}

#[derive(Deserialize)]
struct PersonalizedDocumentData {
    id: String,
}

#[derive(Deserialize)]
struct SemanticSearchResponse {
    documents: Vec<PersonalizedDocumentData>,
}

//FIXME Once the "old" version we test migration against is the version where this
//      test was added we can simplify it a lot by using the Silo API and the additional
//      integration test utils added in this and the previous PR.
#[test]
fn test_full_migration() {
    use old_xayn_web_api::{
        config as old_config,
        start as start_old,
        Ingestion as OldIngestion,
        Personalization as OldPersonalization,
        ELASTIC_MAPPING,
    };

    run_async_test(|test_id| async move {
        let (pg_config, es_config) = legacy_test_setup(&test_id).await?;
        let config = build_test_config_from_parts(&pg_config, &es_config, Table::new());

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        let es_mapping: Value = serde_json::from_str(ELASTIC_MAPPING)?;
        // the old setup didn't setup elastic search itself, nor has the config pub fields
        send_assert(
            &client,
            client
                .put(format!("{}/{}", es_config.url, es_config.index_name))
                .json(&es_mapping)
                .build()?,
            StatusCode::OK,
        )
        .await;

        let args = &[
            "integration-test",
            "--bind-to",
            "127.0.0.1:0",
            "--config",
            &format!("inline:{config}"),
        ];

        let config = old_config::load_with_args([""; 0], args);
        let ingestion = start_old::<OldIngestion>(config).await?;
        let ingestion_url = ingestion.url();
        let config = old_config::load_with_args([""; 0], args);
        let personalization = start_old::<OldPersonalization>(config).await?;
        let personalization_url = personalization.url();

        send_assert(
            &client,
            client
                .post(ingestion_url.join("/documents")?)
                .json(&json!({
                    "documents": [
                        { "id": "d1", "snippet": "snippet 1" },
                        { "id": "d2", "snippet": "snippet 2" }
                    ]
                }))
                .build()?,
            StatusCode::CREATED,
        )
        .await;

        let SemanticSearchResponse { documents } = send_assert_json(
            &client,
            client
                .post(personalization_url.join("/semantic_search")?)
                .json(&json!({ "document": { "query": "snippet" } }))
                .build()?,
            StatusCode::OK,
        )
        .await;
        assert_eq!(
            documents
                .iter()
                .map(|document| document.id.as_str())
                .collect::<HashSet<_>>(),
            ["d1", "d2"].into(),
        );

        ingestion.stop_and_wait(Duration::from_secs(1)).await?;
        personalization
            .stop_and_wait(Duration::from_secs(1))
            .await?;

        let pg_options = pg_config.to_connection_options()?;
        let mut conn = PgConnection::connect_with(&pg_options).await?;
        conn.execute("REVOKE ALL ON SCHEMA public FROM PUBLIC")
            .await?;
        conn.close().await?;

        let config = config::load_with_args([""; 0], args);
        let ingestion = start::<Ingestion>(config).await?;
        let ingestion_url = ingestion.url();
        let config = config::load_with_args([""; 0], args);
        let personalization = start::<Personalization>(config).await?;
        let personalization_url = personalization.url();

        let SemanticSearchResponse { documents } = send_assert_json(
            &client,
            client
                .post(personalization_url.join("/semantic_search")?)
                .json(&json!({ "document": { "query": "snippet" } }))
                .build()?,
            StatusCode::OK,
        )
        .await;
        assert_eq!(
            documents
                .iter()
                .map(|document| document.id.as_str())
                .collect::<HashSet<_>>(),
            ["d1", "d2"].into(),
        );

        send_assert(
            &client,
            client
                .post(ingestion_url.join("/documents")?)
                .json(&json!({
                    "documents": [ { "id": "d3", "snippet": "snippet 3" } ]
                }))
                .build()?,
            StatusCode::CREATED,
        )
        .await;
        let SemanticSearchResponse { documents } = send_assert_json(
            &client,
            client
                .post(personalization_url.join("/semantic_search")?)
                .json(&json!({ "document": { "query": "snippet" } }))
                .build()?,
            StatusCode::OK,
        )
        .await;
        assert_eq!(
            documents
                .iter()
                .map(|document| document.id.as_str())
                .collect::<HashSet<_>>(),
            ["d1", "d2", "d3"].into(),
        );

        ingestion.stop_and_wait().await?;
        personalization.stop_and_wait().await?;

        Ok(())
    });
}
