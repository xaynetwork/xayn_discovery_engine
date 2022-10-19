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
    AppState,
    Config,
    PaginationParams,
    SearchParams,
};

#[get("/search")]
pub(crate) async fn search_get(
    config: Data<Config>,
    app_state: Data<AppState>,
    client: Data<Client>,
    search_params: Query<SearchParams>,
) -> Result<impl Responder, BackendError> {
    handle_search(&config, &app_state, &client, &search_params).await
}

#[post("/search")]
pub(crate) async fn search_post(
    config: Data<Config>,
    app_state: Data<AppState>,
    client: Data<Client>,
    search: Json<SearchParams>,
) -> Result<impl Responder, BackendError> {
    handle_search(&config, &app_state, &client, &search).await
}

#[get("/popular")]
pub(crate) async fn popular_get(
    config: Data<Config>,
    app_state: Data<AppState>,
    client: Data<Client>,
    page_params: Query<PaginationParams>,
) -> Result<impl Responder, BackendError> {
    handle_popular(&config, &app_state, &client, &page_params).await
}

#[post("/popular")]
pub(crate) async fn popular_post(
    config: Data<Config>,
    app_state: Data<AppState>,
    client: Data<Client>,
    page_params: Json<PaginationParams>,
) -> Result<impl Responder, BackendError> {
    handle_popular(&config, &app_state, &client, &page_params).await
}

async fn handle_search(
    config: &Config,
    app_state: &Data<AppState>,
    client: &Client,
    search_params: &SearchParams,
) -> Result<impl Responder, BackendError> {
    let response = fetch_search_results(config, app_state, client, search_params).await?;
    Ok(Json(newscatcher::Response::from(response)))
}

async fn handle_popular(
    config: &Config,
    app_state: &Data<AppState>,
    client: &Client,
    params: &PaginationParams,
) -> Result<impl Responder, BackendError> {
    let response = fetch_popular_results(config, app_state, client, params).await?;
    Ok(Json(newscatcher::Response::from(response)))
}

async fn fetch_search_results(
    config: &Config,
    app_state: &Data<AppState>,
    client: &Client,
    search_params: &SearchParams,
) -> Result<elastic::Response<elastic::Article>, BackendError> {
    let history = app_state.history.read().await.clone();

    let body = json!({
        "query": {
            "bool": {
                "filter": [
                    {
                        "more_like_this": {
                            "fields": ["Title", "Abstract"],
                            "like": search_params.query,
                            "min_term_freq": 1,
                            "max_query_terms": 12,
                        }
                    },
                    {"ids": {"values": history}},
                ]
            }
        }
    });

    query_elastic_search(config, client, body).await
}

async fn fetch_popular_results(
    config: &Config,
    app_state: &Data<AppState>,
    client: &Client,
    _params: &PaginationParams,
) -> Result<elastic::Response<elastic::Article>, BackendError> {
    let from_index = app_state.from_index.read().await.clone();

    let body = json!({
        "size": 200,
        "query": {
            "match_all":{}
        },
        "search_after": from_index, // TODO should be conditional
        "sort": [
            {
                "date_published": {
                    "order": "asc"
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

    let url = format!("{}/_search", config.mind_endpoint);

    debug!("Querying '{}'", url);

    let res = client
        .post(url)
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

//impl From<(elastic::Response<elastic::Article>, &PaginationParams)> for newscatcher::Response {
impl From<elastic::Response<elastic::Article>> for newscatcher::Response {
    //fn from((response, params): (elastic::Response<elastic::Article>, &PaginationParams)) -> Self {
    fn from(response: elastic::Response<elastic::Article>) -> Self {
        let total_pages = if response.hits.hits.is_empty() {
            0
        } else {
            // let mut total_pages = response.hits.total.value / params.page_size();
            // if response.hits.total.value % params.page_size() > 0 {
            //     total_pages += 1
            // }
            // total_pages
            let total = response.hits.total.value;
            // let pg_size = params.page_size;
            let pg_size = 200; // TEMP FIXME
            match (total / pg_size, total % pg_size) {
                (pages, 0) => pages,
                (pages, _) => pages + 1,
            }
        };

        let articles = convert(response.hits.hits);
        Self::new(articles, total_pages)
    }
}
