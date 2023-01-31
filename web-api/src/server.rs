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

mod app_state;

use actix_web::web::ServiceConfig;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use tracing::error;

pub(crate) use self::app_state::AppState;
use crate::{logging, logging::init_tracing, net, storage};

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
}

pub type SetupError = anyhow::Error;

/// Run the server with using given endpoint configuration functions.
///
/// The return value is the exit code which should be used.
pub async fn run<A>() -> Result<(), SetupError>
where
    A: Application + 'static,
{
    async {
        let config: A::Config = crate::config::load(&[A::NAME, "XAYN_WEB_API"]);
        init_tracing(config.as_ref());
        let app_state = AppState::<A>::create(config).await?;
        net::run_actix_server(app_state, A::configure_service).await?;
        Ok(())
    }
    .await
    .map_err(|err| {
        error!(%err, "running service failed");
        err
    })
}
