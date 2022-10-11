use actix_web::{
    get,
    post,
    web::{Data, Json, Query},
    Responder,
};
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};

use crate::{
    elastic::{self, Hit},
    errors::BackendError,
    newscatcher,
    Config,
    PaginationParams,
    Search,
    SearchParams,
};

#[get("/search")]
pub(crate) async fn search_get(
    config: Data<Config>,
    client: Data<Client>,
    search_params: Query<SearchParams>,
    page_params: Query<PaginationParams>,
) -> Result<impl Responder, BackendError> {
    handle_search(&config, &client, &search_params, &page_params).await
}

#[post("/search")]
pub(crate) async fn search_post(
    config: Data<Config>,
    client: Data<Client>,
    search: Json<Search>,
) -> Result<impl Responder, BackendError> {
    handle_search(&config, &client, &search.query, &search.pagination).await
}

#[get("/latest-headlines")]
pub(crate) async fn latest_headlines_get(
    config: Data<Config>,
    client: Data<Client>,
    page_params: Query<PaginationParams>,
) -> Result<impl Responder, BackendError> {
    handle_latest_headlines(&config, &client, &page_params).await
}

#[post("/latest-headlines")]
pub(crate) async fn latest_headlines_post(
    config: Data<Config>,
    client: Data<Client>,
    page_params: Json<PaginationParams>,
) -> Result<impl Responder, BackendError> {
    handle_latest_headlines(&config, &client, &page_params).await
}

async fn handle_search(
    config: &Config,
    client: &Client,
    search_params: &SearchParams,
    page_params: &PaginationParams,
) -> Result<impl Responder, BackendError> {
    let response = fetch_search_results(config, client, search_params, page_params).await?;
    Ok(Json(newscatcher::Response::from((response, page_params))))
}

async fn handle_latest_headlines(
    config: &Config,
    client: &Client,
    params: &PaginationParams,
) -> Result<impl Responder, BackendError> {
    let response = fetch_latest_headlines(config, client, params).await?;
    Ok(Json(newscatcher::Response::from((response, params))))
}

async fn fetch_search_results(
    config: &Config,
    client: &Client,
    search_params: &SearchParams,
    page_params: &PaginationParams,
) -> Result<elastic::Response<elastic::Article>, BackendError> {
    let from = (page_params.page() - 1) * page_params.page_size();
    let body = json!({
        "from": from,
        "size": page_params.page_size(),
        "query": {
            "query_string": {
                "query": search_params.query,
                "fields": ["excerpt", "title"]
            }
        }
    });
    query_elastic_search(config, client, body).await
}

async fn fetch_latest_headlines(
    config: &Config,
    client: &Client,
    params: &PaginationParams,
) -> Result<elastic::Response<elastic::Article>, BackendError> {
    let from = (params.page() - 1) * params.page_size();
    let body = json!({
        "from": from,
        "size": params.page_size(),
        "query": {
            "match_all":{}
        },
        "sort": [
            {
                "published_date": {
                    "order": "desc"
                }
            }
        ]
    });
    query_elastic_search(config, client, body).await
}

async fn query_elastic_search(
    config: &Config,
    client: &Client,
    body: Value,
) -> Result<elastic::Response<elastic::Article>, BackendError> {
    debug!("Query: {:#?}", body);

    let url = format!(
        "{}/{}/_search",
        config.elastic_url, config.elastic_index_name
    );

    debug!("Querying '{}'", url);

    let res = client
        .post(url)
        .basic_auth(&config.elastic_user, Some(&config.elastic_password))
        .json(&body)
        .send()
        .await
        .map_err(BackendError::Elastic)?
        .error_for_status()
        .map_err(BackendError::Elastic)?;

    res.json().await.map_err(BackendError::Receiving)
}

fn convert(input: Vec<Hit<elastic::Article>>) -> Vec<newscatcher::Article> {
    input.into_iter().map(Hit::into).collect()
}

impl From<Hit<elastic::Article>> for newscatcher::Article {
    fn from(hit: Hit<elastic::Article>) -> Self {
        Self {
            title: hit.source.title,
            score: None,
            rank: 0,
            clean_url: hit.source.clean_url,
            excerpt: hit.source.excerpt,
            link: hit.source.link,
            media: "".to_string(),
            topic: hit.source.topic,
            country: hit.source.country,
            language: hit.source.language,
            published_date: hit.source.published_date,
            embedding: hit.source.embedding,
        }
    }
}

impl From<(elastic::Response<elastic::Article>, &PaginationParams)> for newscatcher::Response {
    fn from((response, params): (elastic::Response<elastic::Article>, &PaginationParams)) -> Self {
        let total_pages = if response.hits.hits.is_empty() {
            0
        } else {
            // let mut total_pages = response.hits.total.value / params.page_size();
            // if response.hits.total.value % params.page_size() > 0 {
            //     total_pages += 1
            // }
            // total_pages
            let total = response.hits.total.value;
            let pg_size = params.page_size;
            match (total / pg_size, total % pg_size) {
                (pages, 0) => pages,
                (pages, _) => pages + 1,
            }
        };

        let articles = convert(response.hits.hits);
        Self::new(articles, total_pages)
    }
}
