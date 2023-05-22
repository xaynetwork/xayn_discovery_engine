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

//! Provide various utilities for writing integration tests (mainly for web-api).
//!
//! As this is for testing many of the functions here will panic on failure instead
//! propagating an error and then panicking. We still use the `Panic` error type to
//! make it easier to change error handling in the future.
//!
//! Code in this module hard codes the dummy username and password used by local only
//! integration testing.

use std::{
    env,
    future::Future,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::{Arc, Once},
};

use anyhow::{anyhow, bail, Error};
use chrono::Utc;
use once_cell::sync::Lazy;
use rand::random;
use reqwest::{header::HeaderMap, Client, Request, Response, StatusCode, Url};
use secrecy::ExposeSecret;
use serde::de::DeserializeOwned;
use sqlx::{Connection, Executor, PgConnection};
use toml::{toml, Table, Value};
use tracing::{error_span, instrument, Instrument};
use tracing_subscriber::filter::LevelFilter;
use xayn_test_utils::{env::clear_env, error::Panic};
use xayn_web_api::{config, logging, start, AppHandle, Application};
use xayn_web_api_db_ctrl::{LegacyTenantInfo, Silo};
use xayn_web_api_shared::{
    elastic,
    postgres::{self, QuotedIdentifier},
    request::TenantId,
};

/// Absolute path to the root of the project as determined by `just`.
pub static PROJECT_ROOT: Lazy<PathBuf> =
    Lazy::new(|| just(&["_test-project-root"]).unwrap().into());

/// `true` if it runs in a container (e.g. github action)
///
/// This is needed as in containers e.g. on github services
/// will be provided externally while locally we will start them
/// on the fly. Furthermore on github services are only reachable
/// through their dns short name but locally they are only
/// reachable through localhost. Lastly to allow multiple test
/// environments ports locally may differ but in a container
/// should not.
pub static RUNS_IN_CONTAINER: Lazy<bool> = Lazy::new(|| {
    //FIXME more generic detection
    env::var("GITHUB_ACTIONS") == Ok("true".into())
});

/// DB name used for the db we use to create other dbs.
pub const MANAGEMENT_DB: &str = "xayn";

/// Runs `just` with given arguments returning `stdout` as string.
///
/// If just outputs non utf-8 bytes or can't be called or fails this
/// will panic.
///
/// This will capture stdout, but not stderr so warnings, errors, traces
/// and similar will be printed like normal. In case it fails it will also
/// print the previously captured stdout.
pub fn just(args: &[&str]) -> Result<String, anyhow::Error> {
    let Output { status, stdout, .. } = Command::new("just")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;

    let output = String::from_utf8(stdout)?;
    if status.success() {
        Ok(output)
    } else {
        bail!("Cmd `just` failed! Output:\n{output}Just Exit Status: {status}");
    }
}

pub async fn send_assert(client: &Client, req: Request, expected: StatusCode) -> Response {
    let method = req.method().clone();
    let target = req.url().clone();
    let response = client.execute(req).await.unwrap();
    let status = response.status();
    if status != expected {
        let bytes = response.bytes().await.unwrap();
        let text = String::from_utf8_lossy(&bytes);
        panic!(
            "Failed to {method} {target}, status `{status}` instead of `{expected}`.\nBody: `{text}`\n"
        );
    }
    response
}

pub async fn send_assert_json<O>(client: &Client, req: Request, expected: StatusCode) -> O
where
    O: DeserializeOwned,
{
    let method = req.method().clone();
    let target = req.url().clone();
    let response = send_assert(client, req, expected).await;
    let bytes = response.bytes().await.unwrap();
    match serde_json::from_slice::<O>(&bytes) {
        Ok(out) => out,
        Err(err) => {
            let text = String::from_utf8_lossy(&bytes);
            panic!("Failed to decode body of {method} {target}, error: {err}\nbody: {text}")
        }
    }
}

/// Initialize logging in tests.
pub fn initialize_test_logging() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        logging::initialize(&logging::Config {
            file: None,
            level: LevelFilter::WARN,
            // FIXME If we have json logging do fix the panic logging hook
            //       to also log the backtrace instead of disabling the
            //       panic hook.
            install_panic_hook: false,
        })
        .unwrap();
    });
}

/// Wrapper around integration test code which makes sure they run in a semi-isolated context.
///
/// Before anything this function assures two things:
/// - the environment is cleared
/// - if not on CI the necessary services are started (Elastic Search, Postgres)
///
/// Then for each test:
///
/// - a elastic search index is created
/// - a postgres db is created
/// - a service based on `A: Application` is started on it's own port
/// - the config is pre-populated with the elastic search, embedding and postgres info
///   - you can update it using the `configure` callback
/// - the service info including an url to the application is passed to the test
pub async fn test_app<A, F>(
    configure: Option<Table>,
    test: impl FnOnce(Arc<Client>, Arc<Url>, Services) -> F,
) where
    F: Future<Output = Result<(), Panic>>,
    A: Application + 'static,
{
    initialize_test_logging();

    let (configure, enable_legacy_tenant) =
        configure_with_enable_legacy_tenant_for_test(configure.unwrap_or_default());

    let services = setup_web_dev_services(enable_legacy_tenant).await.unwrap();

    let handle = start_test_application::<A>(&services, configure).await;

    test(
        build_client(&services),
        Arc::new(handle.url()),
        services.clone(),
    )
    .await
    .unwrap();

    handle.stop_and_wait().await.unwrap();

    services.cleanup_test().await.unwrap();
}

/// Like `test_app` but runs two applications in the same test context.
pub async fn test_two_apps<A1, A2, F>(
    configure_first: Option<Table>,
    configure_second: Option<Table>,
    test: impl FnOnce(Arc<Client>, Arc<Url>, Arc<Url>, Services) -> F,
) where
    F: Future<Output = Result<(), Panic>>,
    A1: Application + 'static,
    A2: Application + 'static,
{
    initialize_test_logging();

    let (configure_first, first_wit_legacy) =
        configure_with_enable_legacy_tenant_for_test(configure_first.unwrap_or_default());
    let (configure_second, second_with_legacy) =
        configure_with_enable_legacy_tenant_for_test(configure_second.unwrap_or_default());
    assert_eq!(first_wit_legacy, second_with_legacy);

    let services = setup_web_dev_services(first_wit_legacy).await.unwrap();
    let first_handle = start_test_application::<A1>(&services, configure_first).await;
    let second_handle = start_test_application::<A2>(&services, configure_second).await;

    test(
        build_client(&services),
        Arc::new(first_handle.url()),
        Arc::new(second_handle.url()),
        services.clone(),
    )
    .await
    .unwrap();
    let (res1, res2) = tokio::join!(first_handle.stop_and_wait(), second_handle.stop_and_wait());
    res1.expect("first application to not fail during shutdown");
    res2.expect("second application to not fail during shutdown");

    services.cleanup_test().await.unwrap();
}

fn configure_with_enable_legacy_tenant_for_test(mut config: Table) -> (Table, bool) {
    let value = config
        .get("tenants")
        .and_then(|config| config.get("enable_legacy_tenant"))
        .and_then(|value| value.as_bool())
        // This is a different default value then used for deployments.
        .unwrap_or_default();

    extend_config(
        &mut config,
        toml! {
            [tenants]
            enable_legacy_tenant = value
        },
    );

    (config, value)
}

fn build_client(services: &Services) -> Arc<Client> {
    let mut default_headers = HeaderMap::default();
    default_headers.insert(
        "X-Xayn-Tenant-Id",
        services.test_id.as_str().try_into().unwrap(),
    );
    Arc::new(
        Client::builder()
            .default_headers(default_headers)
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap(),
    )
}

pub const UNCHANGED_CONFIG: Option<Table> = None;

pub fn extend_config(current: &mut Table, extension: Table) {
    for (key, value) in extension {
        if let Some(current) = current.get_mut(&key) {
            match (current, value) {
                (Value::Table(current), Value::Table(value)) => extend_config(current, value),
                (current, value) => *current = value,
            }
        } else {
            current.insert(key, value);
        }
    }
}

pub async fn start_test_application<A>(services: &Services, configure: Table) -> AppHandle
where
    A: Application + 'static,
{
    let config = build_test_config_from_parts(
        services.silo.postgres_config(),
        services.silo.elastic_config(),
        configure,
    );

    let args = &[
        "integration-test",
        "--bind-to",
        "127.0.0.1:0",
        "--config",
        &format!("inline:{config}"),
    ];

    let config = config::load_with_args([""; 0], args);

    start::<A>(config)
        .instrument(error_span!("test", test_id = %services.test_id))
        .await
        .unwrap()
}

pub fn build_test_config_from_parts(
    pg_config: &postgres::Config,
    es_config: &elastic::Config,
    configure: Table,
) -> Table {
    let pg_password = pg_config.password.expose_secret().as_str();
    let pg_config = Value::try_from(pg_config).unwrap();
    let es_config = Value::try_from(es_config).unwrap();

    let mut config = toml! {
        [storage]
        postgres = pg_config
        elastic = es_config

        [embedding]
        directory = "../assets/smbert_v0003"
    };

    //the password was serialized as REDACTED in to_toml_value
    extend_config(
        &mut config,
        toml! {
            [storage.postgres]
            password = pg_password
        },
    );

    extend_config(&mut config, configure);
    config
}

/// Generates an ID for the test.
///
/// The format is `YYMMDD_HHMMSS_RRRR` where `RRRR` is a random (16bit) 0 padded hex number.
pub fn generate_test_id() -> String {
    let date = Utc::now().format("%y%m%d_%H%M%S");
    let random = random::<u16>();
    format!("t{date}_{random:0>4x}")
}

#[derive(Clone, Debug)]
pub struct Services {
    /// Id of the test.
    pub test_id: String,
    /// Silo management API
    pub silo: Silo,
    /// Id of the auto generated tenant
    pub tenant_id: TenantId,
}

impl Services {
    #[instrument(skip(self), fields(%test_id=self.tenant_id), err)]
    pub async fn cleanup_test(self) -> Result<(), Error> {
        self.silo.delete_tenant(&self.tenant_id).await?;
        delete_db(self.silo.postgres_config(), MANAGEMENT_DB).await?;
        Ok(())
    }
}

/// Creates a postgres db and elastic search index for running a web-dev integration test.
///
/// A uris usable for accessing the dbs are returned.
async fn setup_web_dev_services(enable_legacy_tenant: bool) -> Result<Services, anyhow::Error> {
    clear_env();
    start_test_service_containers().unwrap();

    let test_id = generate_test_id();
    let tenant_id = TenantId::try_parse_ascii(test_id.as_ref())?;

    let (pg_config, es_config) = db_configs_for_testing(&test_id);

    create_db(&pg_config, MANAGEMENT_DB).await?;

    let default_es_index = es_config.index_name.clone();
    let silo = Silo::new(
        pg_config,
        es_config,
        enable_legacy_tenant.then_some(LegacyTenantInfo {
            es_index: default_es_index,
        }),
    )
    .await?;
    silo.admin_as_mt_user_hack().await?;
    silo.initialize().await?;
    silo.create_tenant(&tenant_id).await?;

    Ok(Services {
        test_id,
        silo,
        tenant_id,
    })
}

pub fn db_configs_for_testing(test_id: &str) -> (postgres::Config, elastic::Config) {
    let pg_db = Some(test_id.to_string());
    let es_index_name = format!("{test_id}_default");
    let pg_config;
    let es_config;
    if *RUNS_IN_CONTAINER {
        pg_config = postgres::Config {
            db: pg_db,
            base_url: "postgres://user:pw@postgres:5432/".into(),
            ..Default::default()
        };
        es_config = elastic::Config {
            url: "http://elasticsearch:9200".into(),
            index_name: es_index_name,
            ..Default::default()
        };
    } else {
        pg_config = postgres::Config {
            db: pg_db,
            base_url: "postgres://user:pw@localhost:3054/".into(),
            ..Default::default()
        };
        es_config = elastic::Config {
            url: "http://localhost:3092".into(),
            index_name: es_index_name,
            ..Default::default()
        }
    }
    (pg_config, es_config)
}

pub async fn create_db(target: &postgres::Config, management_db: &str) -> Result<(), Error> {
    let target_options = target.to_connection_options()?;
    let target_db: QuotedIdentifier = target_options
        .get_database()
        .ok_or_else(|| anyhow!("database needs to be specified"))?
        .parse()?;
    let management_options = target_options.database(management_db);
    let mut conn = PgConnection::connect_with(&management_options).await?;
    let query = format!("CREATE DATABASE {target_db};");
    conn.execute(query.as_str()).await?;
    conn.close().await?;
    Ok(())
}

pub async fn delete_db(target: &postgres::Config, management_db: &str) -> Result<(), Error> {
    let target_options = target.to_connection_options()?;
    let target_db: QuotedIdentifier = target_options
        .get_database()
        .ok_or_else(|| anyhow!("database needs to be specified"))?
        .parse()?;
    let management_options = target_options.database(management_db);
    let mut conn = PgConnection::connect_with(&management_options).await?;
    let query = format!("DROP DATABASE {target_db} WITH (FORCE);");
    conn.execute(query.as_str()).await?;
    conn.close().await?;
    Ok(())
}

/// Start service containers.
///
/// Does nothing on CI where they have to be started from the outside.
pub fn start_test_service_containers() -> Result<(), anyhow::Error> {
    static ONCE: Once = Once::new();
    let mut res = Ok(());
    ONCE.call_once(|| {
        if !std::env::var("CI")
            .map(|value| value == "true")
            .unwrap_or_default()
        {
            res = just(&["web-dev-up"]).map(drop);
        }
    });
    res
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::*;

    #[test]
    fn test_random_id_generation_has_expected_format() -> Result<(), Panic> {
        let regex = Regex::new("^t[0-9]{6}_[0-9]{6}_[0-9a-f]{4}$")?;
        for _ in 0..100 {
            let id = generate_test_id();
            assert!(
                regex.is_match(&id),
                "id does not have expected format: {id:?}",
            );
        }
        Ok(())
    }

    #[test]
    fn test_extend_config_distinct() {
        let mut config = toml! {
            0 = "0"

            [a]
            0 = "a.0"
        };
        extend_config(
            &mut config,
            toml! {
                1 = "1"

                [b]
                0 = "b.0"
            },
        );
        assert_eq!(
            config,
            toml! {
                0 = "0"
                1 = "1"

                [a]
                0 = "a.0"

                [b]
                0 = "b.0"
            },
        );
    }

    #[test]
    fn test_extend_config_subsume() {
        let mut config = toml! {
            0 = "0"

            [a]
            0 = "a.0"

            [a.b]
            0 = "a.b.0"
        };
        extend_config(
            &mut config,
            toml! {
                0 = "00"

                [a]
                1 = "a.1"

                [a.b]
                0 = "a.b.00"

                [a.c]
                0 = "a.c.0"
            },
        );
        assert_eq!(
            config,
            toml! {
                0 = "00"

                [a]
                0 = "a.0"
                1 = "a.1"

                [a.b]
                0 = "a.b.00"

                [a.c]
                0 = "a.c.0"
            },
        );
    }

    #[test]
    fn test_extend_config_mismatch() {
        let mut config = toml! {
            0 = "0"

            [a]
            0 = "a.0"
        };
        extend_config(
            &mut config,
            toml! {
                a = "a"

                [0]
                1 = "0.1"
            },
        );
        assert_eq!(
            config,
            toml! {
                a = "a"

                [0]
                1 = "0.1"
            },
        );
    }
}
