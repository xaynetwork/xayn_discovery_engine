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
    future::Future,
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};

use actix_cors::Cors;
use actix_web::{
    dev::ServerHandle,
    middleware,
    web::{self, JsonConfig, ServiceConfig},
    App,
    HttpResponse,
    HttpServer,
};
use futures_util::{future::BoxFuture, FutureExt};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::{task::JoinHandle, time::timeout};
use tracing::info;

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

pub(crate) fn start_actix_server<A, F>(
    app_state: Arc<A>,
    on_shutdown: impl Fn(Arc<A>) -> F + Send + 'static,
    configure_services: impl Fn(&mut ServiceConfig) + Clone + Send + 'static,
) -> Result<AppHandle, anyhow::Error>
where
    A: AsRef<Config> + Send + Sync + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    let &Config {
        bind_to,
        max_body_size,
        keep_alive,
        client_request_timeout,
    } = (*app_state).as_ref();

    let json_config = JsonConfig::default().limit(max_body_size);
    let web_app_state = web::Data::from(app_state.clone());

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web_app_state.clone())
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
    .bind(bind_to)?;

    let addresses = server.addrs();
    for addr in &addresses {
        info!(bound_to=%addr);
    }

    let server = server.run();
    let server_handle = server.handle();
    // The `server_handle` needs to be polled continuously to work correctly, to achieve this we
    // `spawn` it on the tokio runtime. This hands off the responsibility to poll the server to
    // tokio. At the same time we keep the `JoinHandle` so that we can wait for the server to
    // stop and get it's return value (we don't have to await `term_handle`).
    let term_handle = tokio::spawn(server);
    Ok(AppHandle {
        on_shutdown: Box::new(move || on_shutdown(app_state).boxed()),
        server_handle,
        addresses,
        term_handle,
    })
}

/// A handle to the running application/server.
///
/// It is recommended to call [`AppHandle.wait_for_termination()`] instead
/// of dropping this type to make sure you do not discard the result of the
/// application execution.
#[must_use = "If one of the `Self` consuming methods like [`AppHandle.wait_for_termination()`] isn't called  the `on_shutdown` callback is not run"]
pub struct AppHandle {
    on_shutdown: Box<dyn FnOnce() -> BoxFuture<'static, ()>>,
    server_handle: ServerHandle,
    addresses: Vec<SocketAddr>,
    term_handle: JoinHandle<Result<(), io::Error>>,
}

impl AppHandle {
    /// Returns an [`Url`] under which this app should be reachable.
    #[allow(clippy::missing_panics_doc)]
    pub fn url(&self) -> Url {
        // There is always at least 1 address and formatting it as
        // below is always a syntactically valid Url.
        let addr = self.addresses().first().unwrap();
        Url::parse(&format!("http://{addr}/")).unwrap()
    }

    /// Returns the addresses the server is listening on.
    ///
    /// This is useful if the `net.bind_to` config was set
    /// to a 0-port (e.g. `127.0.0.1:0`) which will make the
    /// os choose a port and this method is the only way to
    /// know the port which was chosen.
    pub fn addresses(&self) -> &[SocketAddr] {
        &self.addresses
    }

    /// Stops the app gracefully and escalates to non-graceful stopping on timeout, then awaits the apps result.
    pub async fn stop_and_wait(self, graceful_timeout: Duration) -> Result<(), anyhow::Error> {
        if timeout(graceful_timeout, self.stop(true)).await.is_err() {
            self.stop(false).await;
        }
        self.wait_for_termination().await
    }

    /// Stops the application.
    ///
    /// To make sure the application is fully stopped and to handle the result of
    /// the application execution you needs to await [`AppHandle.wait_for_termination()`]
    /// afterwards.
    pub async fn stop(&self, graceful: bool) {
        self.server_handle.stop(graceful).await;
    }

    /// Waits for the server/app to have stopped and returns it's return value.
    ///
    /// It is recommended but not required to call this.
    pub async fn wait_for_termination(self) -> Result<(), anyhow::Error> {
        self.term_handle.await??;
        (self.on_shutdown)().await;
        Ok(())
    }
}
