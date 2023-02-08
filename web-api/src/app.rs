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

use std::{env::current_dir, path::PathBuf, sync::Arc};

use actix_web::web::ServiceConfig;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use tracing::info;

pub(crate) use self::state::AppState;
use crate::{
    logging,
    logging::init_tracing,
    net::{self, AppHandle},
    storage,
};

#[async_trait]
pub trait Application {
    const NAME: &'static str;

    type Config: AsRef<logging::Config>
        + AsRef<net::Config>
        + AsRef<storage::Config>
        + DeserializeOwned
        + Serialize
        + Send
        + Sync
        + 'static;
    type Extension: Send + Sync + 'static;
    type Storage: Send + Sync + 'static;

    /// Configures the actix service(s) used by this application.
    ///
    /// This should mainly be used to mount the right routes and
    /// application specific middleware.
    fn configure_service(config: &mut ServiceConfig);

    /// Create an application specific extension to app state.
    //Design Note: We could handle this by adding `TyFrom<&Config<..>>` bounds
    //             to `Extension` but using this helper method is simpler
    //             and it is also easier to add async if needed (using #[async-trait]).
    fn create_extension(config: &Self::Config) -> Result<Self::Extension, SetupError>;

    async fn setup_storage(config: &storage::Config) -> Result<Self::Storage, SetupError>;

    async fn close_storage(state: &Self::Storage);
}

pub type SetupError = anyhow::Error;

/// Run the server with using given endpoint configuration functions.
///
/// The return value is the exit code which should be used.
pub async fn start<A>(config: A::Config) -> Result<AppHandle, SetupError>
where
    A: Application + 'static,
{
    init_tracing(config.as_ref());

    let pwd = current_dir().unwrap_or_else(|_| PathBuf::from("<no working directory set>"));
    info!(pwd=?pwd);

    let app_state = Arc::new(AppState::<A>::create(config).await?);

    net::start_actix_server(app_state, AppState::<A>::close, A::configure_service)
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
        [name, "XAYN_WEB_API".to_string()]
    }};
}