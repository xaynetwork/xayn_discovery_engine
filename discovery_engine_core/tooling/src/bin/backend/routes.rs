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

use actix_web::{
    get,
    post,
    web::{Data, Json, Query},
    Responder,
};
use elastic::SearchResponse;
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};

use crate::{
    elastic::{self, Hit},
    errors::BackendError,
    newscatcher,
    AppState,
    Config,
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
) -> Result<impl Responder, BackendError> {
    handle_popular(&config, &app_state, &client).await
}

#[post("/popular")]
pub(crate) async fn popular_post(
    config: Data<Config>,
    app_state: Data<AppState>,
    client: Data<Client>,
) -> Result<impl Responder, BackendError> {
    handle_popular(&config, &app_state, &client).await
}

async fn handle_search(
    config: &Config,
    app_state: &Data<AppState>,
    client: &Client,
    search_params: &SearchParams,
) -> Result<impl Responder, BackendError> {
    let response = fetch_search_results(config, app_state, client, search_params).await?;
    Ok(Json(newscatcher::Response::from((
        response,
        app_state.page_size,
        app_state.total,
    ))))
}

async fn handle_popular(
    config: &Config,
    app_state: &Data<AppState>,
    client: &Client,
) -> Result<impl Responder, BackendError> {
    let response = fetch_popular_results(config, app_state, client).await?;
    let hits = response.hits.hits.as_slice();
    if hits.is_empty() {
        return Ok(Json(newscatcher::Response::new(Vec::new(), 0)));
    }

    let mut index = app_state.index.write().await;
    let mut from_index = app_state.from_index.write().await;

    if *index + app_state.page_size > app_state.total {
        *index = 0;
        from_index.clear();
    } else {
        *index += app_state.page_size;
        *from_index = hits.last().unwrap(/* nonempty hits */).sort.clone();
    };

    let mut ids = hits.iter().cloned().map(|hit| hit.id).collect();
    let mut history = app_state.history.write().await;
    history.append(&mut ids);

    // TODO may need to reverse list
    Ok(Json(newscatcher::Response::from((
        response,
        app_state.page_size,
        app_state.total,
    ))))
}

async fn fetch_search_results(
    config: &Config,
    app_state: &Data<AppState>,
    client: &Client,
    search_params: &SearchParams,
) -> Result<elastic::SearchResponse<elastic::Article>, BackendError> {
    let query_lower = search_params.query.to_lowercase();
    let history = app_state.history.read().await.clone();
    let index = app_state.index.read().await;

    if *index == 0 || history.is_empty() {
        return Err(BackendError::NoHistory);
    }

    let body = json!({
        "query": {
            "bool": {
                "filter": [
                    {
                        "more_like_this": {
                            "fields": ["Title", "Abstract"],
                            "like": query_lower,
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
) -> Result<elastic::SearchResponse<elastic::Article>, BackendError> {
    let from_index = app_state.from_index.read().await.clone();

    let body = json!({
        "size": app_state.page_size,
        "query": {
            "match_all":{}
        },
        "search_after": from_index, // TODO omitted if empty
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
) -> Result<elastic::SearchResponse<elastic::Article>, BackendError> {
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
            clean_url: hit.source.url.clone(),
            excerpt: hit.source.snippet,
            link: hit.source.url,
            media: "".to_string(),
            topic: hit.source.category,
            country: "".to_string(),
            language: "".to_string(),
            published_date: hit.source.date_published,
            embedding: Vec::new(),
        }
    }
}

impl From<(SearchResponse<elastic::Article>, usize, usize)> for newscatcher::Response {
    fn from(
        (response, page_size, total): (SearchResponse<elastic::Article>, usize, usize),
    ) -> Self {
        let total_pages = if response.hits.hits.is_empty() {
            0
        } else {
            match (total / page_size, total % page_size) {
                (pages, 0) => pages,
                (pages, _) => pages + 1,
            }
        };

        let articles = convert(response.hits.hits);
        Self::new(articles, total_pages)
    }
}
