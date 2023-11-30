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
use tracing::{info, instrument};
use xayn_integration_tests::{
    build_test_config_from_parts_and_names,
    create_db,
    db_configs_for_testing,
    run_async_test,
    send_assert,
    send_assert_json,
    start_test_service_containers,
    TestId,
    MANAGEMENT_DB,
    TEST_EMBEDDING_SIZE,
};
use xayn_test_utils::{asset::ort_target, env::clear_env};
use xayn_web_api::{config, start, Application, Ingestion, Personalization};
use xayn_web_api_db_ctrl::{elastic_create_tenant, LegacyTenantInfo, Silo};
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

        let es_client = elastic::Client::new(es_config.clone())?;
        let legacy_elastic_index_as_tenant_id =
            TenantId::try_parse_ascii(es_config.index_name.as_bytes())?;
        elastic_create_tenant(
            &es_client.with_index(&legacy_elastic_index_as_tenant_id),
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
            TEST_EMBEDDING_SIZE,
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
            TEST_EMBEDDING_SIZE,
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
        info!("entered async test");

        let (pg_config, es_config) = legacy_test_setup(&test_id).await?;
        let ingestion_config = build_test_config_from_parts_and_names(
            Ingestion::NAME,
            &pg_config,
            &es_config,
            Table::new(),
            "smbert_v0003",
            &format!("ort_v1.15.1/{}", ort_target().unwrap()),
        );

        let personalization_config = build_test_config_from_parts_and_names(
            Personalization::NAME,
            &pg_config,
            &es_config,
            Table::new(),
            "smbert_v0003",
            &format!("ort_v1.15.1/{}", ort_target().unwrap()),
        );

        info!("test setup done");

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
            false,
        )
        .await;

        info!("legacy es setup done");

        let config = old_config::load_with_args(
            [""; 0],
            [
                "integration-test",
                "--bind-to",
                "127.0.0.1:0",
                "--config",
                &format!("inline:{ingestion_config}"),
            ],
        );
        let ingestion = start_old::<OldIngestion>(config).await?;
        info!("started old ingestion");
        let ingestion_url = ingestion.url();

        let config = old_config::load_with_args(
            [""; 0],
            [
                "integration-test",
                "--bind-to",
                "127.0.0.1:0",
                "--config",
                &format!("inline:{personalization_config}"),
            ],
        );
        let personalization = start_old::<OldPersonalization>(config).await?;
        info!("started old personalization");
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
            false,
        )
        .await;

        info!("ingested documents");

        let SemanticSearchResponse { documents } = send_assert_json(
            &client,
            client
                .post(personalization_url.join("/semantic_search")?)
                .json(&json!({ "document": { "query": "snippet" } }))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;
        assert_eq!(
            documents
                .iter()
                .map(|document| document.id.as_str())
                .collect::<HashSet<_>>(),
            ["d1", "d2"].into(),
        );

        info!("checked ingested documents");
        let (res1, res2) = tokio::join!(
            async {
                //FIXME use stop and wait once we test against
                //      a version where this is fixed
                ingestion.stop(false).await;
                ingestion.wait_for_termination().await
            },
            async {
                personalization.stop(false).await;
                personalization.wait_for_termination().await
            }
        );
        res1?;
        res2?;
        info!("stopped old ingestion & personalization");

        let pg_options = pg_config.to_connection_options()?;
        let mut conn = PgConnection::connect_with(&pg_options).await?;
        conn.execute("REVOKE ALL ON SCHEMA public FROM PUBLIC")
            .await?;
        conn.close().await?;

        let config = config::load_with_args([""; 0], {
            let config = build_test_config_from_parts_and_names(
                Ingestion::NAME,
                &pg_config,
                &es_config,
                Table::new(),
                "smbert_v0005",
                &format!("ort_v1.15.1/{}", ort_target().unwrap()),
            );
            &[
                "integration-test",
                "--bind-to",
                "127.0.0.1:0",
                "--config",
                &format!("inline:{config}"),
            ]
        });
        let ingestion = start::<Ingestion>(config).await?;
        info!("started new ingestion");
        let ingestion_url = ingestion.url();
        let config = config::load_with_args([""; 0], {
            let config = build_test_config_from_parts_and_names(
                Personalization::NAME,
                &pg_config,
                &es_config,
                Table::new(),
                "smbert_v0005",
                &format!("ort_v1.15.1/{}", ort_target().unwrap()),
            );
            &[
                "integration-test",
                "--bind-to",
                "127.0.0.1:0",
                "--config",
                &format!("inline:{config}"),
            ]
        });
        let personalization = start::<Personalization>(config).await?;
        info!("started new personalization");
        let personalization_url = personalization.url();

        let SemanticSearchResponse { documents } = send_assert_json(
            &client,
            client
                .post(personalization_url.join("/semantic_search")?)
                .json(&json!({ "document": { "query": "snippet" } }))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;
        assert_eq!(
            documents
                .iter()
                .map(|document| document.id.as_str())
                .collect::<HashSet<_>>(),
            ["d1", "d2"].into(),
        );

        info!("checked ingested documents");

        send_assert(
            &client,
            client
                .post(ingestion_url.join("/documents")?)
                .json(&json!({
                    "documents": [ { "id": "d3", "snippet": "snippet 3" } ]
                }))
                .build()?,
            StatusCode::CREATED,
            false,
        )
        .await;

        info!("ingested additional documents");

        let SemanticSearchResponse { documents } = send_assert_json(
            &client,
            client
                .post(personalization_url.join("/semantic_search")?)
                .json(&json!({ "document": { "query": "snippet" } }))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;
        assert_eq!(
            documents
                .iter()
                .map(|document| document.id.as_str())
                .collect::<HashSet<_>>(),
            ["d1", "d2", "d3"].into(),
        );

        info!("checked documents");
        let (res1, res2) =
            tokio::join!(ingestion.stop_and_wait(), personalization.stop_and_wait(),);
        res1?;
        res2?;
        info!("stopped new ingestion & personalization");

        let hits = send_assert_json::<Value>(
            &client,
            client
                .get(format!(
                    "{}/{}/_search",
                    es_config.url, es_config.index_name
                ))
                .json(&json!({
                    "query": {
                        "match_all": {}
                    },
                    "_source": ["parent"]
                }))
                .build()?,
            StatusCode::OK,
            false,
        )
        .await;

        for document in hits["hits"]["hits"].as_array().unwrap() {
            let id = &document["_id"];
            let parent = &document["_source"]["parent"];
            assert_eq!(id, parent);
        }
        Ok(())
    })
}