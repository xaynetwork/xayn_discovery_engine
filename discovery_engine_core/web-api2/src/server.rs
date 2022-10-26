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

use clap::Parser;
use serde::{de::DeserializeOwned, Serialize};
use tracing::error;

use actix_web::{
    web::{self, JsonConfig, ServiceConfig},
    App,
    HttpResponse,
    HttpServer,
};

use crate::{
    load_config::load_config,
    logging::init_tracing,
    middleware::{json_error::wrap_non_json_errors, tracing::tracing_log_request},
};

pub use self::{
    app_state::AppState,
    config::{Config, NetConfig},
};

pub trait Application {
    type ConfigExtension: DeserializeOwned + Serialize + Send + Sync + 'static;
    fn configure(config: &mut ServiceConfig);
}

pub type SetupError = Box<dyn std::error::Error + 'static>;

/// Run the server with using given endpoint configuration functions.
///
/// The return value is the exit code which should be used.
pub async fn run<A: Application>() -> Result<(), SetupError>
where
    A: Application,
{
    async {
        let mut cli_args = cli::Args::parse();
        let config_file = cli_args.config.take();
        let config = load_config::<Config<A::ConfigExtension>, _>(
            config_file.as_deref(),
            cli_args.to_config_overrides(),
        )?;

        if cli_args.print_config {
            println!("{}", serde_json::to_string_pretty(&config)?);
            return Ok(());
        }

        let addr = config.net.bind_to;
        init_tracing(config.as_ref());

        let json_config = JsonConfig::default().limit(config.net.max_body_size);
        let app_state = AppState::create(config).await?;
        let app_state = web::Data::new(app_state);

        HttpServer::new(move || {
            App::new()
                .app_data(app_state.clone())
                .app_data(json_config.clone())
                .service(web::resource("/health").route(web::get().to(HttpResponse::Ok)))
                .configure(A::configure)
                .wrap_fn(wrap_non_json_errors)
                .wrap_fn(tracing_log_request)
        })
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
