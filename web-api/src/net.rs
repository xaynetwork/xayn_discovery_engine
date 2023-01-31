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
    time::Duration,
};

use actix_cors::Cors;
use actix_web::{
    middleware,
    web::{self, JsonConfig, ServiceConfig},
    App,
    HttpResponse,
    HttpServer,
};
use serde::{Deserialize, Serialize};

use crate::middleware::{json_error::wrap_non_json_errors, tracing::tracing_log_request};

mod serde_duration_as_seconds {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(super) fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(deserializer).map(Duration::from_secs)
    }
}

/// Configuration for roughly network/connection layer specific configurations.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Address to which the server should bind.
    pub(crate) bind_to: SocketAddr,

    /// Max body size limit which should be applied to all endpoints
    pub(crate) max_body_size: usize,

    /// Keep alive timeout in seconds
    #[serde(with = "serde_duration_as_seconds")]
    pub(crate) keep_alive: Duration,

    /// Client request timeout in seconds
    #[serde(with = "serde_duration_as_seconds")]
    pub(crate) client_request_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_to: SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 4252).into(),
            max_body_size: 524_288,
            keep_alive: Duration::from_secs(61),
            client_request_timeout: Duration::from_secs(0),
        }
    }
}

pub(crate) async fn run_actix_server<A>(
    app_state: A,
    configure_services: impl Fn(&mut ServiceConfig) + Clone + Send + 'static,
) -> Result<(), anyhow::Error>
where
    A: AsRef<Config> + Send + Sync + 'static,
{
    let &Config {
        bind_to,
        max_body_size,
        keep_alive,
        client_request_timeout,
    } = app_state.as_ref();

    let json_config = JsonConfig::default().limit(max_body_size);
    let app_state = web::Data::new(app_state);

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .app_data(json_config.clone())
            .service(web::resource("/health").route(web::get().to(HttpResponse::Ok)))
            .configure(&configure_services)
            .wrap_fn(wrap_non_json_errors)
            .wrap_fn(tracing_log_request)
            .wrap(middleware::Compress::default())
            .wrap(Cors::permissive())
    })
    .keep_alive(keep_alive)
    .client_request_timeout(client_request_timeout)
    .bind(bind_to)?
    .run()
    .await
    .map_err(Into::into)
}
