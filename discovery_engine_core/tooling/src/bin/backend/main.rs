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

mod elastic;
mod errors;
mod newscatcher;
mod routes;

use actix_web::{middleware::Logger, web, App, HttpServer};
use envconfig::Envconfig;
use errors::BackendError;
use log::info;
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::routes::{popular_get, popular_post, search_get, search_post};

#[derive(Envconfig, Clone, Debug)]
pub struct Config {
    #[envconfig(from = "MIND_ENDPOINT")]
    pub mind_endpoint: String,
}

struct AppState {
    index: RwLock<usize>,
    from_index: RwLock<String>,
    history: RwLock<Vec<String>>,
    page_size: usize,
    total: usize,
}

#[derive(Deserialize, Debug)]
struct SearchParams {
    #[serde(rename(deserialize = "q"))]
    query: String,
}

async fn query_count(
    config: &Config,
    client: &Client,
) -> Result<elastic::CountResponse, BackendError> {
    let url = format!("{}/_count", config.mind_endpoint);

    let res = client
        .post(url)
        .send()
        .await
        .map_err(BackendError::Elastic)?
        .error_for_status()
        .map_err(BackendError::Elastic)?;

    res.json().await.map_err(BackendError::Receiving)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let config = Config::init_from_env().expect("Could not read config from environment");
    let client = Client::new();

    let response = query_count(&config, &client)
        .await
        .expect("Could not query count from elastic search");

    let app_state = web::Data::new(AppState {
        index: RwLock::new(0),
        from_index: RwLock::new(String::new()),
        history: RwLock::new(Vec::new()),
        page_size: 200,
        total: response.count,
    });

    let app_config = web::Data::new(config);
    let app_client = web::Data::new(client);

    let addr = "0.0.0.0";
    let port = 8080;
    info!("Starting server on {addr}:{port}");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(app_config.clone())
            .app_data(app_state.clone())
            .app_data(app_client.clone())
            .service(search_get)
            .service(search_post)
            .service(popular_get)
            .service(popular_post)
    })
    .bind((addr, port))?
    .run()
    .await
}
