// Copyright 2021 Xayn AG
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

use std::{
    cmp::Ordering,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use displaydoc::Display;
use futures::{
    pin_mut,
    stream::{FuturesUnordered, Stream, StreamExt, TryStreamExt},
};
use rand_distr::{Beta, BetaError, Distribution};
use thiserror::Error;
use tokio::sync::RwLock;

use crate::utils::nan_safe_f32_cmp;

#[derive(Error, Debug, Display)]
pub enum Error {
    /// Error while sampling.
    Sampling(#[from] BetaError),
    /// No items left in a [`Bucket`].
    EmptyBucket,
    /// No [`Bucket`] to pull from.
    NoBucketsToPull,
}

pub(crate) trait BetaSample {
    fn sample(&self, alpha: f32, beta: f32) -> Result<f32, Error>;
}

/// Sample a value from a beta distribution.
pub(crate) struct BetaSampler;

impl BetaSample for BetaSampler {
    fn sample(&self, alpha: f32, beta: f32) -> Result<f32, Error> {
        Ok(Beta::new(alpha, beta)?.sample(&mut rand::thread_rng()))
    }
}

pub(crate) trait Bucket<T> {
    /// Returns the alpha parameter of the beta distribution.
    fn alpha(&self) -> f32;
    /// Returns the beta parameter of the beta distribution.
    fn beta(&self) -> f32;
    /// Checks if the bucket is empty.
    fn is_empty(&self) -> bool;
    /// Removes the next best element from this bucket and returns it, or `None` if it is empty.
    fn pop(&mut self) -> Option<T>;
}

/// Samples the next element from the buckets.
#[allow(clippy::future_not_send)]
async fn pull_arms<BS, B, T>(beta_sampler: &BS, buckets: &[&RwLock<B>]) -> Option<Result<T, Error>>
where
    BS: BetaSample,
    B: Bucket<T>,
{
    match buckets
        .iter()
        .map(|&bucket| async move { bucket })
        .collect::<FuturesUnordered<_>>()
        .filter_map(|bucket| async move {
            let br = bucket.read().await;
            (!br.is_empty()).then(|| {
                beta_sampler
                    .sample(br.alpha(), br.beta())
                    .map(|sample| (sample, bucket))
            })
        })
        .try_fold(None, |max, current| async move {
            if let Some((max_sample, _)) = max {
                if let Ordering::Greater = nan_safe_f32_cmp(&current.0, &max_sample) {
                    Ok(Some(current))
                } else {
                    Ok(max)
                }
            } else {
                Ok(Some(current))
            }
        })
        .await
    {
        Ok(Some((_, bucket))) => bucket.write().await.pop().map(Ok),
        Ok(None) => None,
        Err(error) => Some(Err(error)),
    }
}

/// A stream to select elements from buckets.
pub(crate) struct Selection<'b, BS, B, T> {
    beta_sampler: BS,
    buckets: Vec<&'b RwLock<B>>,
    bucket_type: PhantomData<T>,
}

impl<'b, BS, B, T> Selection<'b, BS, B, T> {
    /// Creates a selective steam.
    pub(crate) fn new<I>(beta_sampler: BS, buckets: I) -> Self
    where
        I: IntoIterator<Item = &'b RwLock<B>>,
    {
        Self {
            beta_sampler,
            buckets: buckets.into_iter().collect(),
            bucket_type: PhantomData,
        }
    }

    /// Selects n elements.
    #[allow(clippy::future_not_send)]
    pub(crate) async fn select(self, n: usize) -> Result<Vec<T>, Error>
    where
        BS: BetaSample,
        B: Bucket<T>,
    {
        self.take(n).try_collect().await
    }
}

impl<'b, BS, B, T> Stream for Selection<'b, BS, B, T>
where
    BS: BetaSample,
    B: Bucket<T>,
{
    type Item = Result<T, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next = pull_arms(&self.beta_sampler, &self.buckets);
        pin_mut!(next);
        next.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::RwLock;

    use super::*;

    struct MockBetaSampler;

    impl BetaSample for MockBetaSampler {
        fn sample(&self, alpha: f32, beta: f32) -> Result<f32, Error> {
            Ok(alpha - beta)
        }
    }

    struct Stack {
        alpha: f32,
        beta: f32,
        docs: Vec<u32>,
    }

    impl Bucket<u32> for Stack {
        fn alpha(&self) -> f32 {
            self.alpha
        }

        fn beta(&self) -> f32 {
            self.beta
        }

        fn is_empty(&self) -> bool {
            self.docs.is_empty()
        }

        fn pop(&mut self) -> Option<u32> {
            self.docs.pop()
        }
    }

    #[tokio::test]
    async fn test_selection() {
        let stack_0 = RwLock::new(Stack {
            alpha: 0.01,
            beta: 1.0,
            docs: vec![],
        });
        let stack_1 = RwLock::new(Stack {
            alpha: 0.01,
            beta: 1.0,
            docs: vec![0],
        });
        let stack_2 = RwLock::new(Stack {
            alpha: 1.0,
            beta: 0.001,
            docs: vec![1, 2, 3],
        });
        let stack_3 = RwLock::new(Stack {
            alpha: 0.001,
            beta: 1.0,
            docs: vec![4, 5, 6],
        });

        let docs = Selection::new(
            MockBetaSampler,
            vec![&stack_0, &stack_1, &stack_2, &stack_3],
        )
        .select(10)
        .await
        .unwrap();

        assert_eq!(docs[0], 3);
        assert_eq!(docs[1], 2);
        assert_eq!(docs[2], 1);
        assert_eq!(docs[3], 0);
        assert_eq!(docs[4], 6);
        assert_eq!(docs[5], 5);
        assert_eq!(docs[6], 4);
    }
}
