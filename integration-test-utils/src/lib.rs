// Copyright 2021 Xayn AG
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
//! propagating an error and then panicing. We still use the `Panic` error type out
//! of making it easier to change error handling in the future.
//!
//! Code in this module hard codes the dummy username and password used by local only
//! integration testing.

use std::{
    fs,
    future::Future,
    path::PathBuf,
    pin::Pin,
    process::{Command, Output, Stdio},
    sync::Mutex,
    time::Duration,
};

use chrono::Local;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Method;
use scopeguard::{guard_on_success, OnSuccess, ScopeGuard};
use sqlx::{Connection, PgConnection};
use tokio::time::sleep;
use xayn_ai_test_utils::error::Panic;

/// Absolute path to the root of the project as determined by `just`.
pub static PROJECT_ROOT: Lazy<PathBuf> = Lazy::new(|| just(&["project-root"]).unwrap().into());

/// Runs `just` with given arguments returning `stdout` as string.
///
/// If just outputs non utf-8 bytes or can't be called or fails this
/// will panic.
///
/// This will capture stdout, but not stderr so warnings, errors, traces
/// and similar will be printed like normal. In case it fails it will also
/// print the previously captured stdout.
pub fn just(args: &[&str]) -> Result<String, Panic> {
    let Output { status, stdout, .. } = Command::new("just")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;

    let output = String::from_utf8(stdout)?;
    if status.success() {
        Ok(output)
    } else {
        panic!(
            "Just failed! Output:\n{}Just Exit Status: {}",
            output, status
        );
    }
}

/// Generates an ID for the test.
///
/// The format is `YYMMDD_HHMMSS_RRRR` where `RRRR` is a random (16bit) 0 padded hex number.
pub fn generate_test_id() -> String {
    let now = Local::now();
    format!(
        "t{}_{:04x}",
        now.format("%y%m%d_%H%M%S"),
        rand::random::<u16>()
    )
}

/// Creates a postgres db for running a web-dev integration test.
///
/// A uri usable for accessing the db is returned.
pub async fn create_web_dev_pg_db(
    name: &str,
) -> Result<ScopeGuard<String, impl FnOnce(String), OnSuccess>, Panic> {
    let uri = create_database(name).await?;
    let name = name.to_owned();
    Ok(guard_on_success(uri, move |_| {
        tokio::spawn(async move {
            drop_database(&name).await.ok();
        });
    }))
}

async fn create_database(name: &str) -> Result<String, Panic> {
    let mut db = PgConnection::connect("postgresql://user:pw@localhost/xayn").await?;

    sqlx::query(&format!("CREATE DATABASE {name};"))
        .execute(&mut db)
        .await?;

    Ok(format!("postgresql://user:pw@localhost/{}", name))
}

async fn drop_database(name: &str) -> Result<(), Panic> {
    let mut db = PgConnection::connect("postgresql://user:pw@localhost/xayn").await?;

    sqlx::query(&format!("DROP DATABASE {name};"))
        .execute(&mut db)
        .await?;

    Ok(())
}

/// Creates a elastic search index for running a web-dev integration test.
///
/// A uri usable for accessing the index is returned.
pub async fn create_web_dev_es_index(
    name: &str,
) -> Result<ScopeGuard<String, impl FnOnce(String), OnSuccess>, Panic> {
    let es_index_uri = format!("http://localhost:9200/{}", name);

    let mut ready = false;
    for _ in 0..30 {
        if check_if_es_ready(&es_index_uri).await {
            ready = true;
            break;
        }
        sleep(Duration::new(1, 0)).await;
    }
    if !ready {
        panic!("Elastic Search is not accessible at: {}", es_index_uri);
    }

    create_index(&es_index_uri).await?;

    Ok(guard_on_success(es_index_uri, |uri| {
        tokio::spawn(async move {
            drop_index(&uri).await.ok();
        });
    }))
}

/// Returns true if Elastic Search is ready.
///
/// The URI should be to a potential index, it is okay if the index doesn't exist,
/// but it should not be to the elastic search root uri.
pub async fn check_if_es_ready(es_index_uri: &str) -> bool {
    let res = reqwest::Client::new()
        .request(Method::OPTIONS, es_index_uri)
        .send()
        .await;

    res.map(|res| res.status().is_success()).unwrap_or(false)
}

async fn create_index(es_index_uri: &str) -> Result<(), Panic> {
    let mapping = fs::read(PROJECT_ROOT.join("./web-api/elastic-search/mapping.json"))?;
    let response = reqwest::Client::new()
        .put(es_index_uri)
        .header("Content-Type", mime::APPLICATION_JSON.as_ref())
        .body(mapping)
        .send()
        .await?;

    if response.status().is_success() {
        Ok(())
    } else {
        let body = response.text().await?;
        panic!("Creating index ({es_index_uri}) failed: {body}");
    }
}

async fn drop_index(es_index_uri: &str) -> Result<(), Panic> {
    let response = reqwest::Client::new().delete(es_index_uri).send().await?;

    if response.status().is_success() {
        Ok(())
    } else {
        let body = response.text().await?;
        panic!("Dropping index ({es_index_uri}) failed: {body}");
    }
}

/// A struct containing the parameters passed to an web-dev integration test.
pub struct WebDevEnv<'a> {
    /// A (random) unique id generated for this test.
    pub id: &'a str,
    /// A URI allowing access to a postgres db created for this test.
    pub pg_uri: &'a str,
    /// A URI allowing access to an elastic search index created for this test.
    pub es_uri: &'a str,
}

/// Runs given closure in a context where a run specific ES/PG index/db is created.
///
/// - makes sure the environment is cleaned for better reproducibility (e.g. not accidentally
///   inferring with integration tests due to exported variables for local testing)
///   - some environment variables are kept, like `CI` or anything starting with
///     `DOCKER`.
/// - if not on CI: handle background services by calling `just web-dev-up`
/// - generate a test id
/// - create a postgres db for the test
/// - create a elastic search index for the test
/// - call the test
/// - if it doesn't fail
///   - delete the postgres db
///   - delete the elastic search index
pub async fn web_dev_integration_test_setup<T>(
    func: impl for<'a> FnOnce(WebDevEnv<'a>) -> Pin<Box<dyn Future<Output = Result<T, Panic>> + 'a>>,
) -> Result<T, Panic> {
    clear_env();
    if !std::env::var("CI")
        .map(|value| value == "true")
        .unwrap_or_default()
    {
        just(&["web-dev-up"])?;
    }

    let id = generate_test_id();
    eprintln!("TestId={}", id);

    let es_cleanup_guard = create_web_dev_es_index(&id).await?;
    let pg_cleanup_guard = create_web_dev_pg_db(&id).await?;

    let env = WebDevEnv {
        id: &id,
        pg_uri: &pg_cleanup_guard,
        es_uri: &es_cleanup_guard,
    };

    func(env).await
}

/// Remove all variables from this process environment (with some exceptions).
///
/// Exceptions are following variables if well formed:
///
/// - `PATH`
/// - `LANG`
/// - `PWD`
/// - `CI`
///
/// Additional exceptions to avoid potential complications
/// with programs called by just, especially wrt to docker-compose
/// or podman-compose:
///
/// - `DBUS*`
/// - `SYSTEMD*`
/// - `USER*`
/// - `DOCKER*`
/// - `PODMAN*`
/// - `XDG*`
pub fn clear_env() {
    // We need to make sure we only do it once as this can be called concurrently
    // and `remove_var` is not reliably thread safe.
    let _guard = ENV_CLEAR_GUARD.lock();
    for (key, _) in std::env::vars_os() {
        let keep = key
            .to_str()
            .map(|key| ENV_PRUNE_EXCEPTIONS.is_match(key))
            .unwrap_or_default();

        if !keep {
            std::env::remove_var(key)
        }
    }
}

static ENV_CLEAR_GUARD: Mutex<()> = Mutex::new(());
static ENV_PRUNE_EXCEPTIONS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?ix)
        (?:^PATH$)
        |(?:^LANG$)
        |(?:^PWD$)
        |(?:^USER$)
        |(?:^CI$)
        |(?:^HOME$)
        |(?:^DBUS)
        |(?:^SYSTEMD)
        |(?:^DOCKER)
        |(?:^PODMAN)
        |(?:^XDG)
        "#,
    )
    .unwrap()
});

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
                "id does not have expected format: {:?}",
                id
            );
        }
        Ok(())
    }

    #[test]
    fn test_env_filter_reges() {
        let vars = [
            "ANDROID_SDK_HOME",
            "BINARYEN_ROOT",
            "CHROME_DESKTOP",
            "CHROME_EXECUTABL",
            "CLUTTER_BACKEND",
            "COLORTERM",
            "DBUS_SESSION_BUS_ADDRESS",
            "DEBUGINFOD_URLS",
            "DEFAULT",
            "DISPLAY",
            "CI",
            "ECORE_EVAS_ENGINE",
            "EDITOR",
            "ELM_ENGINE",
            "FLUTTER_HOME",
            "GDK_BACKEND",
            "GIT_ASKPASS",
            "GNOME_TERMINAL_SCREEN",
            "GNOME_TERMINAL_SERVICE",
            "GPG_TTY",
            "HOME",
            "I3SOCK",
            "LANG",
            "LOGNAME",
            "MAIL",
            "MOTD_SHOWN",
            "NO_AT_BRIDGE",
            "OLDPWD",
            "ORIGINAL_XDG_CURRENT_DESKTOP",
            "PATH",
            "PULSEMIXER_BAR_STYL",
            "PWD",
            "QT_QPA_PLATFORM",
            "QT_WAYLAND_DISABLE_WINDOWDECORATIONS",
            "SHELL",
            "SHLVL",
            "SSH_AUTH_SOCK",
            "STUDIO_JDK",
            "SWAYSOCK",
            "SYSTEMD_EXEC_PID",
            "TERM",
            "TERM_PROGRAM",
            "TERM_PROGRAM_VERSION",
            "USER",
            "VSCODE_GIT_ASKPASS_EXTRA_ARGS",
            "VSCODE_GIT_ASKPASS_MAIN",
            "VSCODE_GIT_ASKPASS_NODE",
            "VSCODE_GIT_IPC_HANDLE",
            "VTE_VERSION",
            "WAYLAND_DISPLAY",
            "XCURSOR_SIZE",
            "XDG_BIN_HOME",
            "XDG_CACHE_HOME",
            "XDG_CONFIG_DIRS",
            "XDG_CURRENT_DESKTOP",
            "XDG_DATA_DIRS",
            "XDG_DATA_HOME",
            "XDG_RUNTIME_DIR",
            "XDG_SEAT",
            "XDG_SESSION_CLASS",
            "XDG_SESSION_ID",
            "XDG_SESSION_TYPE",
            "XDG_VTNR",
            "DOCKER_DODO",
            "PODMAN_DODO",
            "_JAVA_AWT_WM_NONREPARENTING",
        ];

        let filtered = vars
            .into_iter()
            .filter(|key| ENV_PRUNE_EXCEPTIONS.is_match(key))
            .collect::<Vec<_>>();

        assert_eq!(
            filtered,
            [
                "DBUS_SESSION_BUS_ADDRESS",
                "CI",
                "HOME",
                "LANG",
                "PATH",
                "PWD",
                "SYSTEMD_EXEC_PID",
                "USER",
                "XDG_BIN_HOME",
                "XDG_CACHE_HOME",
                "XDG_CONFIG_DIRS",
                "XDG_CURRENT_DESKTOP",
                "XDG_DATA_DIRS",
                "XDG_DATA_HOME",
                "XDG_RUNTIME_DIR",
                "XDG_SEAT",
                "XDG_SESSION_CLASS",
                "XDG_SESSION_ID",
                "XDG_SESSION_TYPE",
                "XDG_VTNR",
                "DOCKER_DODO",
                "PODMAN_DODO",
            ]
        )
    }
}
