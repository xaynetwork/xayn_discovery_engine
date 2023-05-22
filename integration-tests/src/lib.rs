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
    process::{abort, Command, Output, Stdio},
    sync::{Arc, Once},
    time::Duration,
};

use anyhow::{anyhow, bail, Error};
use chrono::Utc;
use derive_more::{AsRef, Display};
use once_cell::sync::Lazy;
use rand::random;
use reqwest::{header::HeaderMap, Client, Request, Response, StatusCode, Url};
use secrecy::ExposeSecret;
use serde::de::DeserializeOwned;
use sqlx::{Connection, Executor, PgConnection};
use toml::{toml, Table, Value};
use tracing::{dispatcher, error_span, instrument, Dispatch, Instrument};
use tracing_subscriber::fmt::TestWriter;
use xayn_test_utils::{env::clear_env, error::Panic};
use xayn_web_api::{config, start, AppHandle, Application};
use xayn_web_api_db_ctrl::{Silo, Tenant};
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

/// Initializes fallback logging.
///
/// This only exist to make sure all logs are always logged
/// even if there is an accident and the global dispatch is
/// used instead of the per-test dispatch.
///
/// There are a small number of logs where this is always the
/// case (but we also normally don't care about) like when actix
/// logs that it started a new worker thread.
pub fn initialize_test_logging_fallback() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let var = env::var_os("XAYN_TEST_FALLBACK_LOG");
        let directives = var
            .as_deref()
            .map(|s| {
                s.to_str()
                    .expect("XAYN_TEST_FALLBACK_LOG must only contain utf-8")
            })
            .unwrap_or("warn");
        tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(TestWriter::default())
            .with_env_filter(directives)
            .init()
    });
}

static LOG_ENV_FILTER: Lazy<String> = Lazy::new(|| {
    env::var_os("XAYN_TEST_LOG")
        .map(|s| {
            s.to_str()
                .expect("XAYN_TEST_LOG must only contain utf-8")
                .into()
        })
        .unwrap_or_else(|| "info,sqlx::query=warn".into())
});

pub fn initialize_local_test_logging(_test_id: &TestId) -> Dispatch {
    initialize_test_logging_fallback();
    //FIXME create a test{%test_id} span valid for the duration of the Dispatch
    //      and automatically "recreated" on any new threads.
    //FIXME add `XAYN_TEST_WRITE_LOGS` which will write logs as jsons to files
    //      and uses a different env filter (env var `XAYN_TEST_FILE_LOG`).
    //FIXME add support for writing flame graphs
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(TestWriter::default())
        .with_env_filter(LOG_ENV_FILTER.as_str())
        .finish()
        .into()
}

pub fn run_async_with_test_logger<F>(test_id: &TestId, body: F) -> F::Output
where
    F: Future,
{
    let subscriber = initialize_local_test_logging(test_id);

    dispatcher::with_default(&subscriber, || {
        let body = body.instrument(error_span!(parent: None, "test", %test_id));

        // more or less what #[tokio::test] does
        // Hint: If we use a "non-current-thread" runtime in the future
        //       make sure to attach the subscriber to the body future
        //       using `WithSubscriber.with_current_subscriber()`.
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
            .block_on(body)
    })
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
pub fn test_app<A, F>(
    configure: Option<Table>,
    test: impl FnOnce(Arc<Client>, Arc<Url>, Services) -> F,
) where
    F: Future<Output = Result<(), Panic>>,
    A: Application + 'static,
{
    let test_id = &TestId::generate();
    run_async_with_test_logger(test_id, async {
        let (configure, enable_legacy_tenant) =
            configure_with_enable_legacy_tenant_for_test(configure.unwrap_or_default());

        let services = setup_web_dev_services(test_id, enable_legacy_tenant)
            .await
            .unwrap();

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
    })
}

/// Like `test_app` but runs two applications in the same test context.
pub fn test_two_apps<A1, A2, F>(
    configure_first: Option<Table>,
    configure_second: Option<Table>,
    test: impl FnOnce(Arc<Client>, Arc<Url>, Arc<Url>, Services) -> F,
) where
    F: Future<Output = Result<(), Panic>>,
    A1: Application + 'static,
    A2: Application + 'static,
{
    let test_id = &TestId::generate();
    run_async_with_test_logger(test_id, async {
        let (configure_first, first_wit_legacy) =
            configure_with_enable_legacy_tenant_for_test(configure_first.unwrap_or_default());
        let (configure_second, second_with_legacy) =
            configure_with_enable_legacy_tenant_for_test(configure_second.unwrap_or_default());
        assert_eq!(first_wit_legacy, second_with_legacy);

        let services = setup_web_dev_services(test_id, first_wit_legacy)
            .await
            .unwrap();
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
        let (res1, res2) =
            tokio::join!(first_handle.stop_and_wait(), second_handle.stop_and_wait(),);
        res1.expect("first application to not fail during shutdown");
        res2.expect("second application to not fail during shutdown");

        services.cleanup_test().await.unwrap();
    })
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

#[derive(Clone, Debug, Display, AsRef)]
#[as_ref(forward)]
pub struct TestId(String);

impl TestId {
    /// Generates an ID for the test.
    ///
    /// The format is `YYMMDD_HHMMSS_RRRR` where `RRRR` is a random (16bit) 0 padded hex number.
    pub fn generate() -> Self {
        let date = Utc::now().format("%y%m%d_%H%M%S");
        let random = random::<u16>();
        Self(format!("t{date}_{random:0>4x}"))
    }

    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

#[derive(Clone, Debug)]
pub struct Services {
    /// Id of the test.
    pub test_id: TestId,
    /// Silo management API
    pub silo: Silo,
    /// Tenant created for the test
    pub tenant: Tenant,
}

impl Services {
    #[instrument(skip(self), fields(%test_id=self.tenant.tenant_id), err)]
    pub async fn cleanup_test(self) -> Result<(), Error> {
        self.silo
            .delete_tenant(self.tenant.tenant_id.clone())
            .await?;
        delete_db(self.silo.postgres_config(), MANAGEMENT_DB).await?;
        Ok(())
    }
}

/// Creates a postgres db and elastic search index for running a web-dev integration test.
///
/// A uris usable for accessing the dbs are returned.
async fn setup_web_dev_services(
    test_id: &TestId,
    enable_legacy_tenant: bool,
) -> Result<Services, anyhow::Error> {
    clear_env();
    start_test_service_containers();

    let tenant_id = TenantId::try_parse_ascii(test_id.as_ref())?;

    let (pg_config, es_config) = db_configs_for_testing(test_id);

    create_db(&pg_config, MANAGEMENT_DB).await?;

    let silo = Silo::new(
        pg_config, es_config,
        // we create the legacy tenant using the silo API,
        // there are separate tests for the testing the migration
        None,
    )
    .await?;
    silo.admin_as_mt_user_hack().await?;
    silo.initialize().await?;
    let tenant = silo
        .create_tenant(tenant_id.clone(), enable_legacy_tenant)
        .await?;

    Ok(Services {
        test_id: test_id.to_owned(),
        silo,
        tenant,
    })
}

pub fn db_configs_for_testing(test_id: &TestId) -> (postgres::Config, elastic::Config) {
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
pub fn start_test_service_containers() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        if !std::env::var("CI")
            .map(|value| value == "true")
            .unwrap_or_default()
        {
            if let Err(err) = just(&["web-dev-up"]) {
                eprintln!("Can not start web-dev services: {err}");
                abort();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::*;

    #[test]
    fn test_random_id_generation_has_expected_format() -> Result<(), Panic> {
        let regex = Regex::new("^t[0-9]{6}_[0-9]{6}_[0-9a-f]{4}$")?;
        for _ in 0..100 {
            let id = TestId::generate();
            assert!(
                regex.is_match(id.as_str()),
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
