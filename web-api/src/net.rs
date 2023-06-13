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
use futures_util::future::BoxFuture;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tracing::{
    dispatcher,
    info,
    info_span,
    instrument,
    instrument::WithSubscriber,
    Dispatch,
    Instrument,
};
use xayn_web_api_shared::{request::TenantId, serde::serde_duration_as_seconds};

use crate::middleware::{
    json_error::wrap_non_json_errors,
    request_context::setup_request_context,
    tracing::new_http_server_with_subscriber,
};

/// Configuration for roughly network/connection layer specific configurations.
// Hint: this value just happens to be copy, if needed the Copy trait can be removed
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Address to which the server should bind.
    pub(crate) bind_to: SocketAddr,

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
            keep_alive: Duration::from_secs(61),
            client_request_timeout: Duration::from_secs(0),
        }
    }
}

#[instrument(skip_all)]
pub(crate) fn start_actix_server(
    net_config: Config,
    legacy_tenant: Option<TenantId>,
    attach_app: impl Fn(&mut ServiceConfig) + Send + Clone + 'static,
    on_shutdown: Box<dyn FnOnce() -> BoxFuture<'static, ()>>,
) -> Result<AppHandle, anyhow::Error> {
    // limits are handled by the infrastructure
    let json_config = JsonConfig::default().limit(u32::MAX as usize);
    let subscriber = dispatcher::get_default(Dispatch::clone);
    let server = new_http_server_with_subscriber!(subscriber, move || {
        let legacy_tenant = legacy_tenant.clone();
        App::new()
            .service(
                web::resource("/health")
                    .route(web::get().to(HttpResponse::Ok))
                    .wrap(Cors::default()),
            )
            .service({
                web::scope("")
                    .app_data(json_config.clone())
                    .configure(&attach_app)
                    .wrap_fn(wrap_non_json_errors)
                    .wrap_fn(move |r, s| setup_request_context(legacy_tenant.as_ref(), r, s))
                    .wrap(middleware::Compress::default())
                    .wrap(Cors::permissive())
            })
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
    // FIXME: instrument with service name, do same for all requests
    let term_handle = tokio::spawn(
        //Hint: make sure to create span after spawning.
        async move { server.instrument(info_span!("polling_server")).await }
            .with_current_subscriber(),
    );
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
    #[instrument(skip(self))]
    pub async fn stop_and_wait(self) -> Result<(), anyhow::Error> {
        //FIXME find out why graceful shutdown on a idle actix server is broken
        self.stop().await;
        self.wait_for_termination().await
    }

    /// Stops the application.
    ///
    /// To make sure the application is fully stopped and to handle the result of
    /// the application execution you needs to await [`AppHandle.wait_for_termination()`]
    /// afterwards.
    #[instrument(name = "stop_actix_server", skip(self))]
    pub async fn stop(&self) {
        self.server_handle.stop(false).await;
    }

    /// Waits for the server/app to have stopped and returns it's return value.
    ///
    /// It is recommended but not required to call this.
    #[instrument(skip(self))]
    pub async fn wait_for_termination(self) -> Result<(), anyhow::Error> {
        self.term_handle
            .instrument(info_span!("awaiting termination"))
            .await??;
        (self.on_shutdown)()
            .instrument(info_span!("on_shutdown_callback"))
            .await;
        Ok(())
    }
}
