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
use xayn_discovery_engine_providers::Market;

use crate::engine::GenericError;

type ItemsResult<I> = Result<Vec<I>, GenericError>;
type Requests<I> = FuturesUnordered<JoinHandle<ItemsResult<I>>>;

async fn request_new_items<I: Send>(
    requests_fn: impl FnOnce() -> Requests<I> + Send,
) -> ItemsResult<I> {
    let mut requests = requests_fn();
    let mut items = Vec::new();
    let mut error = None;

    while let Some(handle) = requests.next().await {
        // should we also push handle errors?
        if let Ok(result) = handle {
            match result {
                Ok(batch) => items.extend(batch),
                Err(err) => {
                    error.replace(err);
                }
            }
        }
    }

    if items.is_empty() && error.is_some() {
        Err(error.unwrap(/* nonempty error */))
    } else {
        Ok(items)
    }
}

pub(super) async fn request_min_new_items<I: Send>(
    max_requests: u32,
    min_articles: usize,
    requests_fn: impl Fn(u32) -> Requests<I> + Send + Sync,
    filter_fn: impl Fn(Vec<I>) -> ItemsResult<I> + Send + Sync,
) -> ItemsResult<I> {
    let mut items = Vec::new();
    let mut error = None;

    for request_num in 0..max_requests {
        match request_new_items(|| requests_fn(request_num)).await {
            // if the API doesn't return any new items, we stop requesting more pages
            Ok(batch) if batch.is_empty() => break,
            Ok(batch) => items.extend(batch),
            Err(err) => {
                error.replace(err);
            }
        };

        items = filter_fn(items)
            .map_err(|err| error.replace(err))
            .unwrap_or_default();

        if items.len() >= min_articles {
            break;
        }
    }

    if items.is_empty() && error.is_some() {
        Err(error.unwrap(/* nonempty error */))
    } else {
        Ok(items)
    }
}

pub(super) fn create_requests_for_markets<I>(
    markets: Vec<Market>,
    request_fn: impl Fn(Market) -> JoinHandle<ItemsResult<I>> + Send,
) -> Requests<I> {
    markets
        .into_iter()
        .map(request_fn)
        .collect::<FuturesUnordered<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Resp(Result<Vec<u32>, GenericError>);

    impl Resp {
        fn ok(items: &[u32]) -> Self {
            Self(Ok(items.to_owned()))
        }

        fn err(msg: &str) -> Self {
            Self(Err(GenericError::from(msg)))
        }
    }

    fn client(responses: Vec<Resp>) -> Requests<u32> {
        responses
            .into_iter()
            .map(|response| tokio::spawn(async { response.0 }))
            .collect::<FuturesUnordered<_>>()
    }

    #[tokio::test]
    async fn test_request_new_items() {
        let items = request_new_items(|| {
            let responses = vec![Resp::ok(&[1]), Resp::ok(&[2, 3])];
            client(responses)
        })
        .await
        .unwrap();
        assert_eq!(items.len(), 3);
    }

    #[tokio::test]
    async fn test_request_new_items_only_errors() {
        let res = request_new_items(|| {
            let responses = vec![Resp::err("0"), Resp::err("1")];
            client(responses)
        })
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "1");
    }

    #[tokio::test]
    async fn test_request_new_items_mixed() {
        let items = request_new_items(|| {
            let responses = vec![Resp::err("0"), Resp::ok(&[1])];
            client(responses)
        })
        .await
        .unwrap();
        assert_eq!(items.len(), 1);
    }

    #[tokio::test]
    async fn test_request_min_new_items_filter_filter() {
        let items = request_min_new_items(
            3,
            2,
            |i| {
                let responses = vec![Resp::ok(&[i])];
                client(responses)
            },
            |items| Ok(items.into_iter().filter(|item| *item != 2).collect()),
        )
        .await
        .unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_request_min_new_items_filter_error() {
        let res: Result<Vec<u32>, GenericError> = request_min_new_items(
            1,
            1,
            |_| {
                let responses = vec![Resp::ok(&[0])];
                client(responses)
            },
            |_| Err(GenericError::from("filter")),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "filter");
    }

    #[tokio::test]
    async fn test_request_min_new_items() {
        let items = request_min_new_items(
            2,
            2,
            |i| {
                let responses = vec![Resp::ok(&[i])];
                client(responses)
            },
            Ok,
        )
        .await
        .unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_request_min_new_items_only_errors() {
        let res = request_min_new_items(
            2,
            2,
            |i| {
                let responses = vec![Resp::err(&format!("{}", i))];
                client(responses)
            },
            Ok,
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "1");
    }

    #[tokio::test]
    async fn test_request_min_new_items_mixed() {
        let items = request_min_new_items(
            2,
            2,
            |i| {
                let responses = vec![Resp::err(&format!("{}", i)), Resp::ok(&[i])];
                client(responses)
            },
            Ok,
        )
        .await
        .unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_request_min_new_items_less_than_min() {
        let items = request_min_new_items(
            3,
            10,
            |i| {
                let responses = vec![Resp::ok(&[i])];
                client(responses)
            },
            Ok,
        )
        .await
        .unwrap();
        assert_eq!(items.len(), 3);
    }

    #[tokio::test]
    async fn test_request_min_new_items_more_than_min() {
        let items = request_min_new_items(
            3,
            1,
            |i| {
                let responses = vec![Resp::ok(&[i]), Resp::ok(&[i])];
                client(responses)
            },
            Ok,
        )
        .await
        .unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], 0);
    }

    #[tokio::test]
    async fn test_request_min_new_items_no_requests() {
        let items = request_min_new_items(
            0,
            0,
            |i| {
                let responses = vec![Resp::ok(&[i]), Resp::ok(&[i])];
                client(responses)
            },
            Ok,
        )
        .await
        .unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_request_min_new_items_exit_early() {
        let items = request_min_new_items(
            5,
            10,
            |i| {
                if i == 2 {
                    FuturesUnordered::new()
                } else {
                    let responses = vec![Resp::ok(&[i])];
                    client(responses)
                }
            },
            Ok,
        )
        .await
        .unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[1], 1);
    }
}
