mod elastic;
mod errors;
mod newscatcher;
mod routes;

use actix_web::{middleware::Logger, web, App, HttpServer};
use envconfig::Envconfig;
use log::info;
use reqwest::Client;
use serde::Deserialize;

use crate::routes::{latest_headlines_get, latest_headlines_post, search_get, search_post};

#[derive(Envconfig, Clone, Debug)]
pub struct Config {
    #[envconfig(from = "ELASTIC_URL")]
    pub elastic_url: String,

    #[envconfig(from = "ELASTIC_USER")]
    pub elastic_user: String,

    #[envconfig(from = "ELASTIC_PASSWORD")]
    pub elastic_password: String,

    #[envconfig(from = "ELASTIC_INDEX_NAME")]
    pub elastic_index_name: String,
}

#[derive(Deserialize, Debug)]
struct SearchParams {
    #[serde(rename(deserialize = "q"))]
    query: String,
}

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

#[derive(Deserialize, Debug)]
struct PaginationParams {
    #[serde(default = "default_page")]
    page: usize,
    #[serde(default = "default_page_size")]
    page_size: usize,
}

impl PaginationParams {
    pub fn page_size(&self) -> usize {
        self.page_size.clamp(1, 100)
    }

    pub fn page(&self) -> usize {
        self.page.clamp(1, 100)
    }
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    100
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let config = Config::init_from_env().expect("Could not read config from environment");

    let addr = "0.0.0.0";
    let port = 8080;
    info!("Starting server on {}:{}", addr, port);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(Client::new()))
            .service(search_get)
            .service(search_post)
            .service(latest_headlines_get)
            .service(latest_headlines_post)
    })
    .bind((addr, port))?
    .run()
    .await
}
