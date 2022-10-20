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
    #[allow(dead_code)]
    index: RwLock<usize>,
    #[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Search {
    #[serde(flatten)]
    query: SearchParams,

    #[serde(flatten)]
    pagination: PaginationParams,
    // These parameters may be sent by our client, but we will currently
    // ignore these for these, for the POC at least.
    // sort_by: String,
    // lang: String,
    // countries: String,
    // not_sources: String,
    // to_rank: usize,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct PaginationParams {
    #[serde(default = "default_page")]
    page: usize,
    #[serde(default = "default_page_size")]
    page_size: usize,
}

#[allow(dead_code)]
impl PaginationParams {
    pub fn page_size(&self) -> usize {
        self.page_size.clamp(1, 200)
    }

    pub fn page(&self) -> usize {
        self.page.clamp(1, 200)
    }
}

#[allow(dead_code)]
fn default_page() -> usize {
    1
}

#[allow(dead_code)]
fn default_page_size() -> usize {
    200
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

    let addr = "0.0.0.0";
    let port = 8080;
    info!("Starting server on {}:{}", addr, port);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(config.clone()))
            .app_data(app_state.clone())
            .app_data(web::Data::new(client.clone()))
            .service(search_get)
            .service(search_post)
            .service(popular_get)
            .service(popular_post)
    })
    .bind((addr, port))?
    .run()
    .await
}
