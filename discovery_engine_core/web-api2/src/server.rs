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

use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::Path,
};

use clap::Parser;
use serde::Serialize;
use tracing::error;

use actix_web::{
    web::{self, ServiceConfig},
    App,
    HttpServer,
};

use crate::{
    config::{load_config, Config},
    middleware::{json_error::wrap_non_json_errors, tracing::tracing_log_request},
    tracing::init_tracing,
};

pub(crate) fn default_bind_address() -> SocketAddr {
    SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080).into()
}

pub trait Application {
    type Config: Config;
    type AppState: TryFrom<Self::Config> + Send + Sync + 'static;
    fn configure(config: &mut ServiceConfig);
}

/// Server for running the web-api.
#[derive(Parser, Debug, Serialize)]
#[command(author, version, about)]
struct CliArgs {
    /// Host and port to which the server should bind.
    ///
    /// This setting is prioritized over settings through
    /// the config and environment.
    #[arg(short, long)]
    bind_to: Option<SocketAddr>,
}

pub type SetupError = Box<dyn std::error::Error + 'static>;

/// Run the server with using given endpoint configuration functions.
///
/// The return value is the exit code which should be used.
pub async fn run<A: Application>(config_file: Option<&Path>) -> Result<(), SetupError>
where
    A: Application,
    <A::AppState as TryFrom<A::Config>>::Error: std::error::Error,
{
    async {
        let cli_args = CliArgs::parse();
        let config = load_config::<A::Config, _>(config_file, cli_args)?;
        let addr = config.bind_address();
        init_tracing(config.log_file());
        let app_state = web::Data::new(A::AppState::try_from(config)?);

        HttpServer::new(move || {
            App::new()
                .app_data(app_state.clone())
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
