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

use std::{
    fs,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::Mutex,
    time::Duration,
};

use chrono::Local;
use once_cell::sync::Lazy;
use regex::{bytes::Regex as BytesRegex, Regex};
use reqwest::Method;
use scopeguard::{guard_on_success, OnSuccess, ScopeGuard};
use sqlx::{Connection, PgConnection};
use tokio::time::sleep;

use crate::error::Panic;

/// Absolute path to the root of the project;
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
        "{}_{:04x}",
        now.format("%y%m%d_%H%M%S"),
        rand::random::<u16>()
    )
}

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

    sqlx::query("CREATE DATABASE ?;")
        .bind(name)
        .execute(&mut db)
        .await?;

    Ok(format!("postgresql://user:pw@localhost/{}", name))
}

async fn drop_database(name: &str) -> Result<(), Panic> {
    let mut db = PgConnection::connect("postgresql://user:pw@localhost/xayn").await?;

    sqlx::query("DROP DATABASE ?;")
        .bind(name)
        .execute(&mut db)
        .await?;

    Ok(())
}

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
/// The URI should be to a potential index, it is okay
/// if the index doesn't exist, but it should not be
/// to the elastic search root uri.
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

pub struct WebDevEnv<'a> {
    pub id: &'a str,
    pub pg_uri: &'a str,
    pub es_uri: &'a str,
}

/// Runs given closure in a context where a run specific ES/PG index/db is created.
pub async fn web_dev_integration_test_setup<T>(
    func: impl for<'a> FnOnce(WebDevEnv<'a>) -> Result<T, Panic>,
) -> Result<T, Panic> {
    clear_env();
    if !std::env::var("CI")
        .map(|value| value == "true")
        .unwrap_or_default()
    {
        just(&["web_dev_up"])?;
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

    let res = func(env)?;

    Ok(res)
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
        r#"^(?ix)
        (?:PATH)
        |(?:LANG)
        |(?:PWD)
        |(?:USER)
        |(?:CI)
        |(?:DBUS.*)
        |(?:SYSTEMD.*)
        |(?:DOCKER.*)
        |(?:PODMAN.*)
        |(?:XDG.*)
        $"#,
    )
    .unwrap()
});

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::*;

    #[test]
    fn test_random_id_generation_has_expected_format() -> Result<(), Panic> {
        let regex = Regex::new("^[0-9]{6}_[0-9]{6}_[0-9a-f]{4}$")?;
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
}
