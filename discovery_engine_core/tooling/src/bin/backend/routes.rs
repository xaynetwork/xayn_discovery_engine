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
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};

use crate::{
    elastic::{Hit, MindArticle, Response},
    errors::BackendError,
    newscatcher::{Article as NewscatcherArticle, Response as NewscatcherResponse},
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

    // order by oldest to newest
    let converted = convert_response(response, app_state.page_size, app_state.total, false);

    Ok(Json(converted))
}

async fn handle_popular(
    config: &Config,
    app_state: &Data<AppState>,
    client: &Client,
) -> Result<impl Responder, BackendError> {
    let response = fetch_popular_results(config, app_state, client).await?;
    let hits = response.hits.hits.as_slice();
    if hits.is_empty() {
        let empty = NewscatcherResponse::new(Vec::new(), 0);
        return Ok(Json(empty));
    }

    let mut index = app_state.index.write().await;
    let mut from_index = app_state.from_index.write().await;

    if *index + app_state.page_size > app_state.total {
        *index = 0;
        *from_index = None;
    } else {
        *index += app_state.page_size;
        *from_index = hits.last().unwrap(/* nonempty hits */).sort.clone();
    };

    let mut ids = hits.iter().cloned().map(|hit| hit.id).collect();
    let mut history = app_state.history.write().await;
    history.append(&mut ids);

    // order by newest to oldest
    let converted = convert_response(response, app_state.page_size, app_state.total, true);

    Ok(Json(converted))
}

async fn fetch_search_results(
    config: &Config,
    app_state: &Data<AppState>,
    client: &Client,
    search_params: &SearchParams,
) -> Result<Response<MindArticle>, BackendError> {
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
) -> Result<Response<MindArticle>, BackendError> {
    let mut body = json!({
        "size": app_state.page_size,
        "query": {
            "match_all": {}
        },
        "sort": [
            {
                "date_published": {
                    "order": "asc"
                }
            }
        ]
    });

    // after the first search also include search_after\
    let index = app_state.index.read().await;
    let from_index = app_state.from_index.read().await;

    if *index > 0 && from_index.is_some() {
        let value = from_index.as_ref().unwrap(/* we check if Some above */);
        let map = body.as_object_mut().unwrap(/* body is Object */);
        map.insert("search_after".to_string(), value.clone());
        body = json!(map);
    }

    query_elastic_search(config, client, body).await
}

async fn query_elastic_search(
    config: &Config,
    client: &Client,
    body: Value,
) -> Result<Response<MindArticle>, BackendError> {
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

impl From<Hit<MindArticle>> for NewscatcherArticle {
    fn from(hit: Hit<MindArticle>) -> Self {
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

fn convert_response(
    response: Response<MindArticle>,
    page_size: usize,
    total: usize,
    reverse: bool,
) -> NewscatcherResponse {
    let hits = response.hits.hits;
    let total_pages = if hits.is_empty() {
        0
    } else {
        match (total / page_size, total % page_size) {
            (pages, 0) => pages,
            (pages, _) => pages + 1,
        }
    };

    let mut articles = hits.into_iter().map(Hit::into).collect::<Vec<_>>();
    if reverse {
        articles.reverse();
    };

    NewscatcherResponse::new(articles, total_pages)
}
