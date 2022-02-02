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
    ops::Deref,
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

/// Samples the next bucket.
fn pull_arms<T>(
    beta_sampler: &impl BetaSample,
    buckets: Vec<impl Deref<Target = impl Bucket<T>>>,
) -> Option<Result<usize, Error>> {
    match buckets
        .into_iter()
        .enumerate()
        .filter(|(_, bucket)| !bucket.is_empty())
        .try_fold(None, |max, (index, bucket)| {
            beta_sampler
                .sample(bucket.alpha(), bucket.beta())
                .map(|sample| {
                    if let Some((max_sample, _)) = max {
                        if let Ordering::Greater = nan_safe_f32_cmp(&sample, &max_sample) {
                            Some((sample, index))
                        } else {
                            max
                        }
                    } else {
                        Some((sample, index))
                    }
                })
        }) {
        Ok(Some((_, index))) => Some(Ok(index)),
        Ok(None) => None,
        Err(error) => Some(Err(error)),
    }
}

/// A stream to select elements from buckets.
pub(crate) struct Selection<'b, BS, B, T> {
    beta_sampler: BS,
    buckets: Vec<&'b RwLock<B>>,
    bucket_type: PhantomData<&'b T>,
}

impl<'b, BS, B, T> Selection<'b, BS, B, T> {
    /// Creates a selective steam.
    pub(crate) fn new(beta_sampler: BS, buckets: impl IntoIterator<Item = &'b RwLock<B>>) -> Self {
        Self {
            beta_sampler,
            buckets: buckets.into_iter().collect(),
            bucket_type: PhantomData,
        }
    }

    /// Selects up to n elements.
    pub(crate) fn select(
        self,
        n: usize,
    ) -> impl 'b + Future<Output = Result<Vec<T>, Error>> + Send + Sync
    where
        BS: 'b + BetaSample + Send + Sync,
        B: Bucket<T> + Send + Sync,
        T: Send + Sync,
    {
        self.take(n).try_collect()
    }
}

impl<'b, BS, B, T> Stream for Selection<'b, BS, B, T>
where
    BS: BetaSample,
    B: Bucket<T>,
{
    type Item = Result<T, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let buckets = self
            .buckets
            .iter()
            .map(|bucket| bucket.read())
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>();
        pin_mut!(buckets);

        match buckets
            .poll(cx)
            .map(|buckets| pull_arms(&self.beta_sampler, buckets))
        {
            Poll::Ready(Some(Ok(index))) => {
                let bucket = self.buckets[index].write();
                pin_mut!(bucket);
                bucket.poll(cx).map(|mut bucket| bucket.pop().map(Ok))
            }
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
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
