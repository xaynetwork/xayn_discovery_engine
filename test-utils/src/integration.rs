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
};

use chrono::Local;
use once_cell::sync::Lazy;
use scopeguard::{guard_on_success, OnSuccess, ScopeGuard};
use sqlx::{postgres::PgConnectOptions, Connection, PgConnection};

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
    let mut db = PgConnection::connect("postgresql://user:pw@localhost/xayn").await?;

    sqlx::query("CREATE DATABASE ?;")
        .bind(name)
        .execute(&mut db)
        .await?;

    Ok(guard_on_success(
        format!("postgresql://user:pw@localhost/{}", name),
        move |_| {
            let mut db = PgConnection::connect("postgresql://user:pw@localhost/xayn").await?;

            sqlx::query("DROP DATABASE ?;")
                .bind(name)
                .execute(&mut db)
                .await?;
        },
    ))
}

pub async fn create_web_dev_es_index(
    es_index_uri: &str,
) -> Result<ScopeGuard<String, impl FnOnce(String), OnSuccess>, Panic> {
    let ready = (0..30).fold(false, |ready, _| {
        ready || check_if_es_ready(es_index_uri).await
    });

    if !ready {
        panic!("Elastic Search is not accessible at: {}", es_index_uri);
    }

    let mapping = fs::read(PROJECT_ROOT.join("./web-api/elastic-search/mapping.json"))?;
    let response = reqwest::Client::new()
        .put(es_index_uri)
        .header("Content-Type", mime::APPLICATION_JSON.as_ref())
        .body(mapping)
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await?;
        panic!("Creating index ({es_index_uri}) failed: {body}");
    }

    Ok(guard_on_success(es_index_uri, |uri| {
        todo!(/*
                HTTP DELETE uri
            */)
    }))
}

pub struct WebDevEnv<'a> {
    id: &'a str,
    pg_uri: &'a str,
    es_uri: &'a str,
}

pub async fn with_web_dev_env<T>(
    func: impl for<'a> FnOnce(WebDevEnv<'a>) -> Result<T, Panic>,
) -> Result<T, Panic> {
    just(&["web_dev_up"])?;

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
