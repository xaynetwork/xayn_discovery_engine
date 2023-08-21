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
    any::Any,
    env::{self, VarError},
    fs::{create_dir_all, remove_dir_all, OpenOptions},
    future::Future,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{abort, Command, Output, Stdio},
    sync::{Arc, Once},
    thread::panicking,
    time::Duration,
};

use anyhow::{anyhow, bail, Error};
use chrono::Utc;
use derive_more::{AsRef, Deref, Display};
use once_cell::sync::Lazy;
use rand::random;
use reqwest::{header::HeaderMap, Client, Request, Response, StatusCode, Url};
use secrecy::ExposeSecret;
use serde::de::DeserializeOwned;
use sqlx::{Connection, Executor, PgConnection};
use toml::{toml, Table, Value};
use tracing::{
    dispatcher,
    error_span,
    info_span,
    instrument,
    metadata::LevelFilter,
    Dispatch,
    Instrument,
};
use tracing_flame::FlameLayer;
use tracing_log::{log::LevelFilter as LogFacilityLevelFilter, LogTracer};
use tracing_subscriber::{
    fmt::{format::FmtSpan, TestWriter},
    layer::SubscriberExt,
    EnvFilter,
    Layer,
};
use xayn_test_utils::{asset::ort_target, env::clear_env, python::initialize_python};
use xayn_web_api::{config, start, AppHandle, Application, Ingestion};
use xayn_web_api_db_ctrl::{Silo, Tenant};
use xayn_web_api_shared::{
    elastic,
    postgres::{self, QuotedIdentifier},
    request::TenantId,
};

use self::env_vars::*;

/// Module to document env variables which affect testing.
mod env_vars {
    /// The [`EnvFilter`](tracing_subscriber::EnvFilter) directives used for the fallback logging setup.
    ///
    /// This is used if for whatever reason the test specific logging
    /// setup is not used.
    ///
    /// Defaults to `warn`.
    pub(super) const XAYN_TEST_FALLBACK_LOG: &str = "XAYN_TEST_FALLBACK_LOG";

    /// The default [`EnvFilter`](tracing_subscriber::EnvFilter) directives used for logging.
    ///
    /// Defaults to `info,sqlx::query=warn`.
    pub(super) const XAYN_TEST_LOG: &str = "XAYN_TEST_LOG";

    /// The [`EnvFilter`](tracing_subscriber::EnvFilter) directives used for logging to stdout.
    ///
    /// If set to `"true"` then [`XAYN_TEST_LOG`] is used.
    ///
    /// If set to `"false"` logging to stdout is disabled.
    ///
    /// If set to another string the string is used as directives.
    ///
    /// Defaults to `"true"`, `""` is treated as if it's the default.
    pub(super) const XAYN_TEST_STDOUT_LOG: &str = "XAYN_TEST_STDOUT_LOG";

    /// The [`EnvFilter`](tracing_subscriber::EnvFilter) directives used for logging to a file.
    ///
    /// If set to `"true"` then [`XAYN_TEST_LOG`] is used.
    ///
    /// If set to `"false"` logging to a file is disabled.
    ///
    /// If set to another string the string is used as directives.
    ///
    /// Defaults to `"false"`, `""` is treated as if it's the default.
    pub(super) const XAYN_TEST_FILE_LOG: &str = "XAYN_TEST_FILE_LOG";

    /// Select which span events to log, default is none.
    ///
    /// It's a case insensitive comma separated list of span event mask names.
    ///
    /// See [`tracing_subscriber::fmt::format::FmtSpan`] for possible mask names.
    ///
    /// Defaults to `"NONE"`.
    pub(super) const XAYN_TEST_FILE_LOG_SPAN_EVENTS: &str = "XAYN_TEST_FILE_LOG_SPAN_EVENTS";

    /// The [`EnvFilter`](tracing_subscriber::EnvFilter) directives used for creating a flame graph.
    ///
    /// If set to `"true"` then [`XAYN_TEST_LOG`] is used.
    ///
    /// If set to `"false"` no flame graph data will be collected.
    ///
    /// If set to another string the string is used as directives.
    ///
    /// Defaults to `"false"`, `""` is treated as if it's the default.
    pub(super) const XAYN_TEST_FLAME_LOG: &str = "XAYN_TEST_FLAME_LOG";

    /// If set to `"true"` the per-test temp. dir will not be deleted even if the test succeeds.
    pub(super) const XAYN_TEST_KEEP_TEMP_DIRS: &str = "XAYN_TEST_KEEP_TEMP_DIRS";

    /// Used to detect if we run in an action and in turn services are externally provided.
    pub(super) const GITHUB_ACTIONS: &str = "GITHUB_ACTIONS";
}

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
    env::var(GITHUB_ACTIONS) == Ok("true".into())
});

static KEEP_TEMP_DIRS: Lazy<bool> =
    Lazy::new(|| env::var(XAYN_TEST_KEEP_TEMP_DIRS) == Ok("true".into()));

static FILE_LOG_SPAN_EVENTS: Lazy<FmtSpan> = Lazy::new(|| {
    let span_events = env_var_os_with_default(XAYN_TEST_FILE_LOG_SPAN_EVENTS, "");
    span_events
        .split(',')
        .filter(|e| !e.trim().is_empty())
        .fold(FmtSpan::NONE, |mask, span_event| {
            mask | match span_event.to_ascii_uppercase().as_str() {
                "NEW" => FmtSpan::NEW,
                "ENTER" => FmtSpan::ENTER,
                "EXIT" => FmtSpan::EXIT,
                "CLOSE" => FmtSpan::CLOSE,
                "NONE" => FmtSpan::NONE,
                "ACTIVE" => FmtSpan::ACTIVE,
                "FULL" => FmtSpan::FULL,
                other => panic!("Unexpected FmtSpan option: {other}"),
            }
        })
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
pub fn just(args: &[&str]) -> Result<String, Error> {
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

#[instrument(skip_all)]
pub async fn send_assert(
    client: &Client,
    req: Request,
    expected: StatusCode,
    is_deprecated: bool,
) -> Response {
    let method = req.method().clone();
    let target = req.url().clone();
    let response = client.execute(req).await.unwrap();

    let headers = response.headers();
    assert_eq!(
        headers.contains_key("deprecation"),
        is_deprecated,
        "Failed to assert headers: {headers:?}",
    );

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

#[instrument(skip_all)]
pub async fn send_assert_json<O>(
    client: &Client,
    req: Request,
    expected: StatusCode,
    is_deprecated: bool,
) -> O
where
    O: DeserializeOwned,
{
    let method = req.method().clone();
    let target = req.url().clone();
    let response = send_assert(client, req, expected, is_deprecated).await;
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
        let env_filter = env_opt_var_os(XAYN_TEST_FALLBACK_LOG)
            .unwrap_or_else(|| "warn".into())
            .parse::<EnvFilter>()
            .expect("XAYN_TEST_FALLBACK_LOG has invalid EnvFilter directives");

        let max_level = env_filter
            .max_level_hint()
            .max(LOG_FILE_ENV_FILTER.as_ref().and_then(|(_, level)| *level))
            .max(LOG_STDOUT_ENV_FILTER.as_ref().and_then(|(_, level)| *level))
            .max(LOG_FLAME_ENV_FILTER.as_ref().and_then(|(_, level)| *level));

        // WARNING: You can not use `.init()` here as it will setup the log=>tracing
        //          facade in a way which yields incorrect results.
        dispatcher::set_global_default(
            tracing_subscriber::fmt()
                .with_ansi(false)
                .with_writer(TestWriter::default())
                .with_env_filter(env_filter)
                .into(),
        )
        .unwrap();

        let mut builder = LogTracer::builder();
        if let Some(max_level) = max_level {
            let filter = match max_level {
                LevelFilter::TRACE => LogFacilityLevelFilter::Trace,
                LevelFilter::DEBUG => LogFacilityLevelFilter::Debug,
                LevelFilter::INFO => LogFacilityLevelFilter::Info,
                LevelFilter::WARN => LogFacilityLevelFilter::Warn,
                LevelFilter::ERROR => LogFacilityLevelFilter::Error,
                LevelFilter::OFF => LogFacilityLevelFilter::Off,
            };
            builder = builder.with_max_level(filter);
        }
        builder.init().unwrap();
    });
}

fn env_opt_var_os(var: &str) -> Option<String> {
    env::var(var)
        .map_or_else(
            |error| match error {
                VarError::NotPresent => None,
                VarError::NotUnicode(err) => panic!("{var} must only contain utf-8: {err:?}"),
            },
            Some,
        )
        .filter(|v| !v.trim().is_empty())
}

fn env_var_os_with_default(var: &str, default: &str) -> String {
    env_opt_var_os(var).unwrap_or_else(|| default.into())
}

fn select_filter_directives(input: String, default_directives: &str) -> Option<String> {
    match input.as_str() {
        "false" => None,
        "true" => Some(default_directives.to_owned()),
        _ => Some(input),
    }
}

fn directives_with_level_filter(
    directives: String,
    var_name: &str,
) -> (String, Option<LevelFilter>) {
    let hint = directives
        .parse::<EnvFilter>()
        .unwrap_or_else(|err| panic!("{var_name} contains invalid EnvFilter directives: {err}"))
        .max_level_hint();
    (directives, hint)
}

fn additional_env_filter(
    var_name: &str,
    default_state: &str,
    default_value: &str,
) -> Option<(String, Option<LevelFilter>)> {
    select_filter_directives(
        env_var_os_with_default(var_name, default_state),
        default_value,
    )
    .map(|directives| directives_with_level_filter(directives, var_name))
}

static LOG_ENV_FILTER: Lazy<(String, Option<LevelFilter>)> = Lazy::new(|| {
    directives_with_level_filter(
        env_var_os_with_default(XAYN_TEST_LOG, "info,sqlx::query=warn"),
        XAYN_TEST_LOG,
    )
});

static LOG_STDOUT_ENV_FILTER: Lazy<Option<(String, Option<LevelFilter>)>> =
    Lazy::new(|| additional_env_filter(XAYN_TEST_STDOUT_LOG, "true", &LOG_ENV_FILTER.0));

static LOG_FILE_ENV_FILTER: Lazy<Option<(String, Option<LevelFilter>)>> =
    Lazy::new(|| additional_env_filter(XAYN_TEST_FILE_LOG, "false", &LOG_ENV_FILTER.0));

static LOG_FLAME_ENV_FILTER: Lazy<Option<(String, Option<LevelFilter>)>> =
    Lazy::new(|| additional_env_filter(XAYN_TEST_FLAME_LOG, "false", &LOG_ENV_FILTER.0));

pub fn initialize_local_test_logging(test_id: &TestId) -> (Dispatch, impl Any) {
    initialize_test_logging_fallback();
    //FIXME create a test{%test_id} span valid for the duration of the Dispatch
    //      and automatically "recreated" on any new threads.
    //FIXME add support for writing flame graphs

    let subscriber = tracing_subscriber::registry();

    let stdout_log = LOG_STDOUT_ENV_FILTER.as_ref().map(|(filter, _)| {
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(TestWriter::default())
            .with_filter(filter.parse::<EnvFilter>().unwrap())
    });

    let file_log = LOG_FILE_ENV_FILTER.as_ref().map(|(filter, _)| {
        let path = test_id.make_temp_file_path("log.json").unwrap();
        eprintln!("Logs written to: {}", path.display());
        let writer = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)
            .unwrap();
        tracing_subscriber::fmt::layer()
            .with_writer(writer)
            .with_span_events(FILE_LOG_SPAN_EVENTS.clone())
            .json()
            .with_filter(filter.parse::<EnvFilter>().unwrap())
    });

    let (flame_log, guard) = LOG_FLAME_ENV_FILTER
        .as_ref()
        .map(|(filter, _)| {
            let path = test_id.make_artifact_file_path("tracing.folded").unwrap();
            eprintln!("Flamegraph data written to: {}", path.display());
            let (layer, guard) = FlameLayer::with_file(path).unwrap();

            (
                layer.with_filter(filter.parse::<EnvFilter>().unwrap()),
                guard,
            )
        })
        .unzip();

    let dispatch = subscriber
        .with(stdout_log)
        .with(file_log)
        .with(flame_log)
        .into();

    (dispatch, guard)
}

pub fn run_async_test<F>(test: impl FnOnce(TestId) -> F)
where
    F: Future<Output = Result<(), Error>>,
{
    let test_id = TestId::generate();
    let (subscriber, flame_guard) = initialize_local_test_logging(&test_id);

    dispatcher::with_default(&subscriber, || {
        let guard = DeleteTempDirIfNoPanic {
            test_id: test_id.clone(),
            disable_cleanup: *KEEP_TEMP_DIRS,
        };

        let span = error_span!(parent: None, "test", %test_id);
        span.in_scope(|| {
            initialize_python();

            let body = test(test_id);

            // more or less what #[tokio::test] does
            // Hint: If we use a "non-current-thread" runtime in the future
            //       make sure to attach the subscriber to the body future
            //       using `WithSubscriber.with_current_subscriber()`.
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed building the Runtime")
                .block_on(body)
                .unwrap();

            drop(guard);
        });
    });

    drop(flame_guard);
}

struct DeleteTempDirIfNoPanic {
    test_id: TestId,
    disable_cleanup: bool,
}

impl Drop for DeleteTempDirIfNoPanic {
    fn drop(&mut self) {
        if self.disable_cleanup || panicking() {
            let temp_dir = self.test_id.temp_dir();
            if temp_dir.exists() {
                let string = format!("Temp dir was not deleted: {}\n", temp_dir.display());
                if self.disable_cleanup {
                    // intentionally sidestep output capturing
                    io::stdout().write_all(string.as_bytes()).ok();
                } else {
                    print!("{}", string);
                }
            }
        } else {
            self.test_id.remove_temp_dir().unwrap();
        }
    }
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
    F: Future<Output = Result<(), Error>>,
    A: Application + 'static,
{
    run_async_test(|test_id| async move {
        let (configure, enable_legacy_tenant) =
            configure_with_enable_legacy_tenant_for_test(configure.unwrap_or_default());

        let services = setup_web_dev_services(&test_id, enable_legacy_tenant).await?;

        let handle = start_test_application::<A>(&services, configure).await;

        test(
            build_client(&services),
            Arc::new(handle.url()),
            services.clone(),
        )
        .instrument(info_span!("call_test"))
        .await?;

        handle
            .stop_and_wait()
            .instrument(info_span!("shutdown_server"))
            .await?;

        services.cleanup_test().await?;
        Ok(())
    })
}

/// Like `test_app` but runs two applications in the same test context.
pub fn test_two_apps<A1, A2, F>(
    configure_first: Option<Table>,
    configure_second: Option<Table>,
    test: impl FnOnce(Arc<Client>, Arc<Url>, Arc<Url>, Services) -> F,
) where
    F: Future<Output = Result<(), Error>>,
    A1: Application + 'static,
    A2: Application + 'static,
{
    run_async_test(|test_id| async move {
        let (configure_first, first_with_legacy) =
            configure_with_enable_legacy_tenant_for_test(configure_first.unwrap_or_default());
        let (configure_second, second_with_legacy) =
            configure_with_enable_legacy_tenant_for_test(configure_second.unwrap_or_default());
        assert_eq!(first_with_legacy, second_with_legacy);

        let services = setup_web_dev_services(&test_id, first_with_legacy).await?;
        let first_handle = start_test_application::<A1>(&services, configure_first).await;
        let second_handle = start_test_application::<A2>(&services, configure_second).await;

        test(
            build_client(&services),
            Arc::new(first_handle.url()),
            Arc::new(second_handle.url()),
            services.clone(),
        )
        .instrument(info_span!("call_test"))
        .await?;
        let (res1, res2) = tokio::join!(
            first_handle
                .stop_and_wait()
                .instrument(info_span!("shutdown_first_server")),
            second_handle
                .stop_and_wait()
                .instrument(info_span!("shutdown_second_server"))
        );
        res1.expect("first application to not fail during shutdown");
        res2.expect("second application to not fail during shutdown");

        services.cleanup_test().await?;
        Ok(())
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
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap(),
    )
}

pub const UNCHANGED_CONFIG: Option<Table> = None;

pub fn with_dev_options() -> Option<Table> {
    Some(toml! {
        [tenants]
        enable_dev = true
    })
}

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

#[instrument(skip_all)]
pub async fn start_test_application<A>(services: &Services, mut configure: Table) -> AppHandle
where
    A: Application + 'static,
{
    if A::NAME == Ingestion::NAME {
        extend_config(
            &mut configure,
            toml! {
                [ingestion.index_update]
                method = "danger_wait_for_completion"

                [snippet_extractor]
                language = "english"
                chunk_size = 10
                hard_chunk_size_limit = 10
            },
        );
    }
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

/// Embedding size used by the `Embedder` used for testing.
pub const TEST_EMBEDDING_SIZE: usize = 384;

pub fn build_test_config_from_parts_and_names(
    pg_config: &postgres::Config,
    es_config: &elastic::Config,
    configure: Table,
    model_name: &str,
    runtime_name: &str,
) -> Table {
    let pg_password = pg_config.password.expose_secret().as_str();
    let pg_config = Value::try_from(pg_config).unwrap();
    let es_config = Value::try_from(es_config).unwrap();

    // Hint: Relative path doesn't work with `cargo flamegraph`
    let model_dir = PROJECT_ROOT
        .join("assets")
        .join(model_name)
        .display()
        .to_string();
    let runtime_dir = PROJECT_ROOT
        .join("assets")
        .join(runtime_name)
        .display()
        .to_string();

    let mut config = toml! {
        [storage]
        postgres = pg_config
        elastic = es_config

        [embedding]
        type = "pipeline"
        directory = model_dir
        runtime = runtime_dir
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

pub fn build_test_config_from_parts(
    pg_config: &postgres::Config,
    es_config: &elastic::Config,
    configure: Table,
) -> Table {
    build_test_config_from_parts_and_names(
        pg_config,
        es_config,
        configure,
        "xaynia_v0201",
        &format!("ort_v1.15.1/{}", ort_target().unwrap()),
    )
}

#[derive(Clone, Debug, Display, Deref, AsRef)]
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

    pub fn temp_dir(&self) -> PathBuf {
        let mut temp_dir = env::temp_dir();
        temp_dir.push(format!("web-api.{}", self.0));
        temp_dir
    }

    pub fn make_temp_file_path(&self, arg: impl AsRef<Path>) -> Result<PathBuf, io::Error> {
        let mut temp_dir = self.temp_dir();
        create_dir_all(&temp_dir)?;
        temp_dir.push(arg);
        Ok(temp_dir)
    }

    pub fn remove_temp_dir(&self) -> Result<(), io::Error> {
        let temp_dir = self.temp_dir();
        if temp_dir.exists() {
            remove_dir_all(self.temp_dir())
        } else {
            Ok(())
        }
    }

    pub fn make_artifact_file_path(&self, name: &str) -> Result<PathBuf, io::Error> {
        let mut path = PROJECT_ROOT.clone();
        path.push("test-artifacts");
        path.push(format!("web-api.{}", &self.0));
        create_dir_all(&path)?;
        path.push(name);
        Ok(path)
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
#[instrument]
async fn setup_web_dev_services(
    test_id: &TestId,
    enable_legacy_tenant: bool,
) -> Result<Services, Error> {
    clear_env();
    start_test_service_containers();

    let tenant_id = TenantId::try_parse_ascii(test_id.as_ref())?;

    let (pg_config, es_config) = db_configs_for_testing(test_id);

    create_db(&pg_config, MANAGEMENT_DB).await?;

    let silo = Silo::new(
        pg_config,
        es_config,
        // we create the legacy tenant using the silo API,
        // there are separate tests for the testing the migration
        None,
        TEST_EMBEDDING_SIZE,
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

#[instrument]
pub fn db_configs_for_testing(test_id: &TestId) -> (postgres::Config, elastic::Config) {
    let pg_db = Some(test_id.to_string());
    let pg_max_pool_size = 3;
    let es_index_name = format!("{test_id}_default");
    let es_timeout = Duration::from_secs(30);
    let pg_config;
    let es_config;
    if *RUNS_IN_CONTAINER {
        pg_config = postgres::Config {
            db: pg_db,
            base_url: "postgres://user:pw@postgres:5432/".into(),
            max_pool_size: pg_max_pool_size,
            ..Default::default()
        };
        es_config = elastic::Config {
            url: "http://elasticsearch:9200".into(),
            index_name: es_index_name,
            timeout: es_timeout,
            ..Default::default()
        };
    } else {
        pg_config = postgres::Config {
            db: pg_db,
            base_url: "postgres://user:pw@localhost:3054/".into(),
            max_pool_size: pg_max_pool_size,
            ..Default::default()
        };
        es_config = elastic::Config {
            url: "http://localhost:3092".into(),
            index_name: es_index_name,
            timeout: es_timeout,
            ..Default::default()
        }
    }
    (pg_config, es_config)
}

#[instrument(skip(target))]
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

#[instrument]
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
#[instrument]
pub fn start_test_service_containers() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        if !*RUNS_IN_CONTAINER {
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
    fn test_random_id_generation_has_expected_format() -> Result<(), Error> {
        let regex = Regex::new("^t[0-9]{6}_[0-9]{6}_[0-9a-f]{4}$")?;
        for _ in 0..100 {
            let id = TestId::generate();
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
