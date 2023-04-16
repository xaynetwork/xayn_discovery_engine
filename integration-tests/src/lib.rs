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
    future::Future,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::{Arc, Once},
    time::Duration,
};

use anyhow::bail;
use once_cell::sync::Lazy;
use reqwest::{header::HeaderMap, Client, Request, Response, StatusCode, Url};
use scopeguard::{guard_on_success, OnSuccess, ScopeGuard};
use serde::de::DeserializeOwned;
use toml::{toml, Table, Value};
use tracing::{info_span, Instrument};
use uuid::Uuid;
use xayn_test_utils::{env::clear_env, error::Panic};
use xayn_web_api::{config, start, AppHandle, Application};

/// Absolute path to the root of the project as determined by `just`.
pub static PROJECT_ROOT: Lazy<PathBuf> =
    Lazy::new(|| just(&["_test-project-root"]).unwrap().into());

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

const APP_STOP_TIMEOUT: Duration = Duration::from_secs(1);

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
    configure: impl FnOnce(&mut Table),
    test: impl FnOnce(Arc<Client>, Arc<Url>, Services) -> F,
) where
    F: Future<Output = Result<(), Panic>>,
    A: Application + 'static,
{
    let services = setup_web_dev_test_context().await.unwrap();

    let handle = start_test_application::<A>(&services, configure).await;

    test(
        build_client(&services),
        Arc::new(handle.url()),
        services.clone(),
    )
    .await
    .unwrap();

    handle.stop_and_wait(APP_STOP_TIMEOUT).await.unwrap();
}

/// Like `test_app` but runs two applications in the same test context.
pub async fn test_two_apps<A1, A2, F>(
    configure_first: impl FnOnce(&mut Table),
    configure_second: impl FnOnce(&mut Table),
    test: impl FnOnce(Arc<Client>, Arc<Url>, Arc<Url>, Services) -> F,
) where
    F: Future<Output = Result<(), Panic>>,
    A1: Application + 'static,
    A2: Application + 'static,
{
    let services = setup_web_dev_test_context().await.unwrap();
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
    let (res1, res2) = tokio::join!(
        first_handle.stop_and_wait(APP_STOP_TIMEOUT),
        second_handle.stop_and_wait(APP_STOP_TIMEOUT),
    );
    res1.expect("first application to not fail during shutdown");
    res2.expect("second application to not fail during shutdown");
}

fn build_client(_services: &Services) -> Arc<Client> {
    let default_headers = HeaderMap::default();
    //FIXME test tool doesn't yet setup tenants
    // default_headers.insert(
    //     "X-Tenant-Id",
    //     services.tenant_id.as_str().try_into().unwrap(),
    // );
    Arc::new(
        Client::builder()
            .default_headers(default_headers)
            .build()
            .unwrap(),
    )
}

pub fn unchanged_config(_: &mut Table) {}

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

pub async fn start_test_application<A>(
    services: &Services,
    configure: impl FnOnce(&mut Table),
) -> AppHandle
where
    A: Application + 'static,
{
    let (es_url, es_index) = services.elastic_search.as_str().rsplit_once('/').unwrap();
    let pg_url = services.postgres.as_str();

    let mut config = toml! {
        [logging]
        level = "warn"

        [storage.postgres]
        base_url = pg_url

        [storage.elastic]
        url = es_url
        index_name = es_index

        [embedding]
        directory = "../assets/smbert_v0003"

        [tenants]
        enable_legacy_tenant = true
    };

    configure(&mut config);

    let args = &[
        "integration-test",
        "--bind-to",
        "127.0.0.1:0",
        "--config",
        &format!("inline:{config}"),
    ];

    let config = config::load_with_args([0u8; 0], args);

    start::<A>(config)
        .instrument(info_span!("test", test_id = %services.id))
        .await
        .unwrap()
}

/// Generates an ID for the test.
///
/// The format is `YYMMDD_HHMMSS_RRRR` where `RRRR` is a random (16bit) 0 padded hex number.
fn generate_test_id() -> Result<String, anyhow::Error> {
    just(&["_test-generate-id"])
}

#[derive(Clone, Debug)]
pub struct Services {
    /// Id of the test.
    pub id: String,
    /// Uri to a postgres db for this test.
    pub postgres: Url,
    /// Uri to a elastic search db for this test.
    pub elastic_search: Url,
    /// Id of the auto generated tenant
    pub tenant_id: String,
}

/// Creates a postgres db and elastic search index for running a web-dev integration test.
///
/// A uris usable for accessing the dbs are returned.
async fn setup_web_dev_test_context(
) -> Result<ScopeGuard<Services, impl FnOnce(Services), OnSuccess>, anyhow::Error> {
    clear_env();
    start_test_service_containers().unwrap();

    let id = generate_test_id()?;

    let out = just(&["_test-create-dbs", &id])?;
    let mut postgres = None;
    let mut elastic_search = None;
    for line in out.lines() {
        if let Some(url) = line.trim().strip_prefix("PG_URL=") {
            postgres = Some(url.parse().unwrap());
        } else if let Some(url) = line.trim().strip_prefix("ES_URL=") {
            elastic_search = Some(url.parse().unwrap());
        }
    }

    let uris = Services {
        id,
        postgres: postgres.unwrap(),
        elastic_search: elastic_search.unwrap(),
        tenant_id: Uuid::nil().to_string(),
    };

    Ok(guard_on_success(uris, move |uris| {
        just(&["_test-drop-dbs", &uris.id]).unwrap();
    }))
}

/// Start service containers.
///
/// Does nothing on CI where they have to be started from the outside.
fn start_test_service_containers() -> Result<(), anyhow::Error> {
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
            let id = generate_test_id().unwrap();
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
