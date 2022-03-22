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

use futures::{stream::FuturesUnordered, StreamExt};
use tokio::task::JoinHandle;
use xayn_discovery_engine_providers::{Article, Error, Market};

use crate::engine::GenericError;

async fn request_new_items(
    markets: Vec<Market>,
    request_fn: impl Fn(Market) -> JoinHandle<Result<Vec<Article>, Error>> + Send,
    filter_fn: impl FnOnce(Vec<Article>) -> Result<Vec<Article>, GenericError> + Send,
) -> Result<Vec<Article>, GenericError> {
    let mut requests = markets
        .into_iter()
        .map(|market| request_fn(market))
        .collect::<FuturesUnordered<_>>();

    let mut articles = Vec::new();
    let mut error = None;

    while let Some(handle) = requests.next().await {
        // should we also push handle errors?
        if let Ok(result) = handle {
            match result {
                Ok(batch) => articles.extend(batch),
                Err(err) => {
                    error.replace(err.into());
                }
            }
        }
    }

    let articles = filter_fn(articles)
        .map_err(|err| error.replace(err))
        .unwrap_or_default();

    if articles.is_empty() && error.is_some() {
        Err(error.unwrap(/* nonempty error */))
    } else {
        Ok(articles)
    }
}

pub(super) async fn request_min_new_items(
    markets: Vec<Market>,
    max_requests: u32,
    min_articles: usize,
    request_fn: impl Fn(Market, usize) -> JoinHandle<Result<Vec<Article>, Error>> + Send + Sync,
    filter_fn: impl Fn(Vec<Article>) -> Result<Vec<Article>, GenericError> + Send + Sync,
) -> Result<Vec<Article>, GenericError> {
    let mut articles = Vec::new();
    let mut error = None;

    for page in 1..=max_requests as usize {
        match request_new_items(
            markets.clone(),
            |market| request_fn(market, page),
            |articles| filter_fn(articles),
        )
        .await
        {
            Ok(batch) => articles.extend(batch),
            Err(err) => {
                error.replace(err);
            }
        };

        if articles.len() >= min_articles {
            break;
        }
    }

    if articles.is_empty() && error.is_some() {
        Err(error.unwrap(/* nonempty error */))
    } else {
        Ok(articles)
    }
}
