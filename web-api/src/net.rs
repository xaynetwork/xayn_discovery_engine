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
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};

use actix_cors::Cors;
use actix_web::{
    dev::{ServerHandle, ServiceFactory, ServiceRequest, ServiceResponse},
    middleware,
    web::{self, JsonConfig},
    App,
    HttpResponse,
    HttpServer,
};
use futures_util::future::BoxFuture;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::{task::JoinHandle, time::timeout};
use tracing::info;

use crate::{
    middleware::{json_error::wrap_non_json_errors, request_context::setup_request_context},
    tenants,
    utils::serde_duration_as_secs,
};

/// Configuration for roughly network/connection layer specific configurations.
// Hint: this value just happens to be copy, if needed the Copy trait can be removed
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Address to which the server should bind.
    pub(crate) bind_to: SocketAddr,

    /// Keep alive timeout in seconds
    #[serde(with = "serde_duration_as_secs")]
    pub(crate) keep_alive: Duration,

    /// Client request timeout in seconds
    #[serde(with = "serde_duration_as_secs")]
    pub(crate) client_request_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_to: SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 4252).into(),
            keep_alive: Duration::from_secs(61),
            client_request_timeout: Duration::from_secs(0),
        }
    }
}

pub(crate) fn start_actix_server<T>(
    net_config: Config,
    context_config: tenants::Config,
    mk_base_app: impl Fn() -> App<T> + Send + Clone + 'static,
    on_shutdown: Box<dyn FnOnce() -> BoxFuture<'static, ()>>,
) -> Result<AppHandle, anyhow::Error>
where
    T: ServiceFactory<
            ServiceRequest,
            Response = ServiceResponse,
            Config = (),
            Error = actix_web::Error,
            InitError = (),
        > + 'static,
{
    // limits are handled by the infrastructure
    let json_config = JsonConfig::default().limit(u32::MAX as usize);
    let context_config = Arc::new(context_config);

    let server = HttpServer::new(move || {
        mk_base_app()
            .app_data(json_config.clone())
            .service(web::resource("/health").route(web::get().to(HttpResponse::Ok)))
            .wrap_fn(wrap_non_json_errors)
            .wrap_fn({
                let context_config = context_config.clone();
                move |r, s| setup_request_context(&context_config, r, s)
            })
            .wrap(middleware::Compress::default())
            .wrap(Cors::permissive())
    })
    .keep_alive(net_config.keep_alive)
    .client_request_timeout(net_config.client_request_timeout)
    .bind(net_config.bind_to)?;

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
        on_shutdown,
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
