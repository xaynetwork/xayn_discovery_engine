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
mod cli;
mod config;

use actix_cors::Cors;
use actix_web::{
    middleware,
    web::{self, JsonConfig, ServiceConfig},
    App,
    HttpResponse,
    HttpServer,
};
use clap::Parser;
use serde::{de::DeserializeOwned, Serialize};
use tracing::error;

pub use self::config::Config;
pub(crate) use self::{
    app_state::AppState,
    config::{impl_config, NetConfig},
};
use crate::{
    load_config::load_config,
    logging::init_tracing,
    middleware::{json_error::wrap_non_json_errors, tracing::tracing_log_request},
};

pub trait Application {
    const NAME: &'static str;

    type Config: Config + DeserializeOwned + Serialize + Send + Sync + 'static;
    type AppStateExtension: Send + Sync + 'static;

    /// Configures the actix service(s) used by this application.
    ///
    /// This should mainly be used to mount the right routes and
    /// application specific middleware.
    fn configure_service(config: &mut ServiceConfig);

    /// Create an application specific extension to app state.
    //Design Note: We could handle this by adding `TyFrom<&Config<..>>` bounds
    //             to `AppStateExtension` but using this helper method is simpler
    //             and it is also easier to add async if needed (using #[async-trait]).
    fn create_app_state_extension(
        config: &Self::Config,
    ) -> Result<Self::AppStateExtension, SetupError>;
}

pub type SetupError = anyhow::Error;

/// Run the server with using given endpoint configuration functions.
///
/// The return value is the exit code which should be used.
pub async fn run<A>() -> Result<(), SetupError>
where
    A: Application,
{
    async {
        let mut cli_args = cli::Args::parse();
        let config_file = cli_args.config.take();
        let config = load_config::<A::Config, _>(
            A::NAME,
            "XAYN_WEB_API",
            config_file.as_deref(),
            cli_args.to_config_overrides(),
        )?;

        if cli_args.print_config {
            println!("{}", serde_json::to_string_pretty(&config)?);
            return Ok(());
        }

        let addr = config.net().bind_to;
        let keep_alive = config.net().keep_alive;
        let client_request_timeout = config.net().client_request_timeout;
        init_tracing(config.logging());

        let json_config = JsonConfig::default().limit(config.net().max_body_size);
        let app_state = AppState::create(config, A::create_app_state_extension).await?;
        let app_state = web::Data::new(app_state);

        HttpServer::new(move || {
            App::new()
                .app_data(app_state.clone())
                .app_data(json_config.clone())
                .service(web::resource("/health").route(web::get().to(HttpResponse::Ok)))
                .configure(A::configure_service)
                .wrap_fn(wrap_non_json_errors)
                .wrap_fn(tracing_log_request)
                .wrap(middleware::Compress::default())
                .wrap(Cors::permissive())
        })
        .keep_alive(keep_alive)
        .client_request_timeout(client_request_timeout)
        .bind(addr)?
        .run()
        .await?;

        Ok(())
    }
    .await
    .map_err(|err| {
        error!(%err, "running service failed");
        err
    })
}
