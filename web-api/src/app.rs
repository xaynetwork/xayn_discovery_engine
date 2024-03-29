// Copyright 2022 Xayn AG
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

mod state;

use std::{env::current_dir, fmt::Debug, path::PathBuf, sync::Arc};

use actix_web::web::ServiceConfig;
use async_trait::async_trait;
use futures_util::FutureExt;
use serde::{de::DeserializeOwned, Serialize};
use tracing::{info, instrument};

pub(crate) use self::state::{AppState, TenantState};
use crate::{
    config::Config,
    extractor,
    logging,
    net::{self, AppHandle},
    storage,
    tenants,
};

#[async_trait]
pub trait Application: 'static {
    const NAME: &'static str;

    type Config: AsRef<logging::Config>
        + AsRef<net::Config>
        + AsRef<storage::Config>
        + AsRef<tenants::Config>
        + AsRef<extractor::Config>
        + DeserializeOwned
        + Serialize
        + Send
        + Sync
        + Debug
        + 'static;

    /// Configures the actix service(s) used by this application.
    ///
    /// This should mainly be used to mount the right routes and
    /// application specific middleware.
    fn configure_service(config: &mut ServiceConfig);

    /// Configures service(s) used by operations.
    ///
    /// Services defined here will not be wrapped in the normal
    /// application middleware and will not be reachable using anything
    /// which uses CORS.
    fn configure_ops_service(_config: &mut ServiceConfig) {}
}

pub type SetupError = anyhow::Error;

/// Run the server with using given endpoint configuration functions.
///
/// The return value is the exit code which should be used.
#[instrument(skip_all)]
pub async fn start<A>(config: Config) -> Result<AppHandle, SetupError>
where
    A: Application + 'static,
{
    info!({ ?config }, "starting service");

    let pwd = current_dir().unwrap_or_else(|_| PathBuf::from("<no working directory set>"));
    info!(pwd=?pwd);

    let net_config = net::Config::clone(config.as_ref());
    let app_state = Arc::new(AppState::create(config).await?);
    let legacy_tenant = app_state.legacy_tenant().cloned();

    let shutdown = Box::new({
        let app_state = app_state.clone();
        move || async { app_state.close().await }.boxed()
    });

    net::start_actix_server(
        net_config,
        legacy_tenant,
        move |service| app_state.clone().attach_to(service),
        A::configure_service,
        A::configure_ops_service,
        shutdown,
    )
}

/// Generate application names/env prefixes for the given application.
///
/// This is a macro as it uses `env!("CARGO_BIN_NAME")` which needs to be called
/// in the binary build unit and won't work if used in a library. This means
/// for crates with a `lib.rs` and `main.rs` it needs to be in `main.rs` or
/// (sub-)modules of `main.rs` and can't be in `lib.rs` or (sub-)modules of
/// `lib.rs`.
#[macro_export]
macro_rules! application_names {
    () => {{
        let name = env!("CARGO_BIN_NAME").replace("-", "_").to_uppercase();
        let name = if name.starts_with("XAYN_") {
            name
        } else {
            format!("XAYN_{name}")
        };
        [
            name,
            "XAYN_WEB_API".to_string(),
            "XAYN_PERSONALIZATION".to_string(),
            "XAYN_INGESTION".to_string(),
        ]
    }};
}
