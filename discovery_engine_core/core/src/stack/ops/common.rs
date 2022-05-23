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

use tokio::task::JoinHandle;
use xayn_discovery_engine_ai::GenericError;

type ItemsResult<I> = Result<Vec<I>, GenericError>;
type Request<I> = JoinHandle<ItemsResult<I>>;

pub(super) async fn request_min_new_items<I: Send>(
    max_requests: u32,
    min_articles: usize,
    page_size: usize,
    request_fn: impl Fn(u32) -> Request<I> + Send + Sync,
    filter_fn: impl Fn(Vec<I>) -> ItemsResult<I> + Send + Sync,
) -> ItemsResult<I> {
    let mut items = Vec::with_capacity(min_articles);
    let mut error = None;

    for request_num in 0..max_requests {
        match request_fn(request_num).await {
            // if the API doesn't return any new items, we stop requesting more pages
            Ok(Ok(batch)) => {
                if batch.is_empty() {
                    break;
                }

                let batch_size = batch.len();
                items.extend(batch);

                items = filter_fn(items)
                    .map_err(|err| error.replace(err))
                    .unwrap_or_default();

                if items.len() >= min_articles || batch_size < page_size {
                    break;
                }
            }
            Ok(Err(err)) => {
                error.replace(err);
            }
            // should we also push handle errors?
            Err(_) => {}
        };
    }

    if items.is_empty() && error.is_some() {
        Err(error.unwrap(/* nonempty error */))
    } else {
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Response(ItemsResult<u32>);

    impl Response {
        fn ok(items: &[u32]) -> Self {
            Self(Ok(items.to_owned()))
        }

        fn err(msg: &str) -> Self {
            Self(Err(GenericError::from(msg)))
        }

        fn request(self) -> Request<u32> {
            tokio::spawn(async { self.0 })
        }
    }

    #[tokio::test]
    async fn test_request_min_new_items_filter_filter() {
        let items = request_min_new_items(
            3,
            2,
            1,
            |i| Response::ok(&[i]).request(),
            |items| Ok(items.into_iter().filter(|item| *item != 2).collect()),
        )
        .await
        .unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_request_min_new_items_filter_error() {
        let res = request_min_new_items(
            1,
            1,
            1,
            |_| Response::ok(&[0]).request(),
            |_| Err(GenericError::from("filter")),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "filter");
    }

    #[tokio::test]
    async fn test_request_min_new_items() {
        let items = request_min_new_items(2, 2, 1, |i| Response::ok(&[i]).request(), Ok)
            .await
            .unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_request_min_new_items_only_errors() {
        let res =
            request_min_new_items(2, 2, 1, |i| Response::err(&format!("{}", i)).request(), Ok)
                .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "1");
    }

    #[tokio::test]
    async fn test_request_min_new_items_mixed() {
        let items = request_min_new_items(
            2,
            2,
            1,
            |i| {
                match i {
                    0 => Response::err(&format!("{}", i)),
                    1 => Response::ok(&[i]),
                    _ => unreachable!(),
                }
                .request()
            },
            Ok,
        )
        .await
        .unwrap();
        assert_eq!(items.len(), 1);
    }

    #[tokio::test]
    async fn test_request_min_new_items_less_than_min() {
        let items = request_min_new_items(3, 10, 1, |i| Response::ok(&[i]).request(), Ok)
            .await
            .unwrap();
        assert_eq!(items.len(), 3);
    }

    #[tokio::test]
    async fn test_request_min_new_items_more_than_min() {
        let items = request_min_new_items(
            3,
            1,
            2,
            |i| {
                match i {
                    0 => Response::ok(&[i, i + 1]),
                    _ => unreachable!(),
                }
                .request()
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
        let items = request_min_new_items(0, 0, 1, |i| Response::ok(&[i]).request(), Ok)
            .await
            .unwrap();
        assert!(items.is_empty());
    }
}
