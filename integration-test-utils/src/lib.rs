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
    future::Future,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::{Arc, Once},
    time::Duration,
};

use once_cell::sync::Lazy;
use reqwest::{Client, Request, Url};
use scopeguard::{guard_on_success, OnSuccess, ScopeGuard};
use serde::de::DeserializeOwned;
use toml::{toml, Table};
use xayn_ai_test_utils::{env::clear_env, error::Panic};
use xayn_web_api::{config, start, Application};

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

pub async fn send_assert_200_json<O>(client: &Client, req: Request) -> O
where
    O: DeserializeOwned,
{
    let method = req.method().clone();
    let target = req.url().clone();
    let response = client.execute(req).await.unwrap();
    let status = response.status();
    if !status.is_success() {
        let bytes = response.bytes().await.unwrap();
        let text = String::from_utf8_lossy(&bytes);
        panic!("Failed to {method} {target}, status {status}, body: {text}");
    }

    let bytes = response.bytes().await.unwrap();

    match serde_json::from_slice::<O>(&bytes) {
        Ok(out) => out,
        Err(err) => {
            let text = String::from_utf8_lossy(&bytes);
            panic!("Failed to decode body of {method} {target}, error: {err}\nbody: {text}")
        }
    }
}

pub async fn test_app<F, A>(
    configure: impl FnOnce(&mut Table),
    test: impl FnOnce(Arc<Client>, Arc<Url>, Services) -> F,
) -> Result<(), Panic>
where
    F: Future<Output = ()>,
    A: Application,
{
    clear_env();
    start_test_service_containers();

    let services = create_web_dev_services().await?;

    let mut config = toml! {
        [storage.postgres]
        [storage.elastic]
        [embedding]
        directory = "../assets/smbert_v0003"
    };

    let storage = config["storage"].as_table_mut().unwrap();
    storage["postgres"]
        .as_table_mut()
        .unwrap()
        .insert("base_url".into(), services.postgres.into());
    storage["elastic"]
        .as_table_mut()
        .unwrap()
        .insert("url".into(), services.elastic_search.into());

    configure(&mut config);

    let args = &[
        "integration-test",
        "--bind-to",
        "127.0.0.1:0",
        "--config",
        &format!("inline:{config}"),
    ];

    let config = config::load_with_args(&[], args);

    let handle = start::<A>(config).await?;
    let addr = handle.addresses().first().unwrap();
    let uri = Url::parse(&format!("http://{addr}/")).unwrap();
    let client = Client::new();

    test(Arc::new(client), Arc::new(uri), services.clone()).await;

    handle.stop_and_wait(Duration::from_secs(1)).await?;

    Ok(())
}

/// Generates an ID for the test.
///
/// The format is `YYMMDD_HHMMSS_RRRR` where `RRRR` is a random (16bit) 0 padded hex number.
fn generate_test_id() -> Result<String, Panic> {
    just(&["_test-generate-id"])
}

#[derive(Clone, Debug)]
pub struct Services {
    /// Id of the test.
    pub id: String,
    /// Uri to a postgres db for this test.
    pub postgres: String,
    /// Uri to a elastic search db for this test.
    pub elastic_search: String,
}

/// Creates a postgres db and elastic search index for running a web-dev integration test.
///
/// A uris usable for accessing the dbs are returned.
async fn create_web_dev_services(
) -> Result<ScopeGuard<Services, impl FnOnce(Services), OnSuccess>, Panic> {
    let id = generate_test_id()?;

    just(&["_test-create-dbs", &id])?;

    let postgres = format!("postgresql://user:pw@localhost/{id}");
    let elastic_search = format!("http://localhost:9200/{id}");

    let uris = Services {
        id,
        postgres,
        elastic_search,
    };

    Ok(guard_on_success(uris, move |uris| {
        just(&["_test-drop-dbs", &uris.id]).unwrap();
    }))
}

/// Start service containers.
///
/// Does nothing on CI where they have to be started from the outside.
fn start_test_service_containers() -> Result<(), Panic> {
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
            let id = generate_test_id()?;
            assert!(
                regex.is_match(&id),
                "id does not have expected format: {:?}",
                id
            );
        }
        Ok(())
    }
}
