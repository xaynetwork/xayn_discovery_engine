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

use std::{cmp::Ordering, marker::PhantomData};

use displaydoc::Display;
use rand_distr::{Beta, BetaError, Distribution};
use thiserror::Error;

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

    /// Retains only the newest documents, given how many to keep.
    fn retain_newest(&mut self, keep: usize);
}

impl<B, T> Bucket<T> for &mut B
where
    B: Bucket<T>,
{
    fn alpha(&self) -> f32 {
        (**self).alpha()
    }

    fn beta(&self) -> f32 {
        (**self).beta()
    }

    fn is_empty(&self) -> bool {
        (**self).is_empty()
    }

    fn pop(&mut self) -> Option<T> {
        (**self).pop()
    }

    fn retain_newest(&mut self, keep: usize) {
        (**self).retain_newest(keep);
    }
}

/// Samples the next non-empty bucket.
fn pull_arms<'b, T>(
    beta_sampler: &impl BetaSample,
    buckets: impl Iterator<Item = &'b mut impl Bucket<T>>,
) -> Option<Result<&'b mut impl Bucket<T>, Error>> {
    buckets
        .filter(|bucket| !bucket.is_empty())
        .try_fold(None, |max, bucket| {
            beta_sampler
                .sample(bucket.alpha(), bucket.beta())
                .map(|sample| {
                    if let Some((max_sample, _)) = max {
                        if let Ordering::Greater = nan_safe_f32_cmp(&sample, &max_sample) {
                            Some((sample, bucket))
                        } else {
                            max
                        }
                    } else {
                        Some((sample, bucket))
                    }
                })
        })
        .transpose()
        .map(|result| result.map(|(_, bucket)| bucket))
}

/// An iterator to select elements from buckets.
pub(crate) struct SelectionIter<'b, BS, B, T> {
    beta_sampler: BS,
    buckets: Vec<&'b mut B>,
    bucket_type: PhantomData<T>,
}

impl<'b, BS, B, T> SelectionIter<'b, BS, B, T> {
    /// Creates a selective iterator.
    pub(crate) fn new(beta_sampler: BS, buckets: impl IntoIterator<Item = &'b mut B>) -> Self {
        Self {
            beta_sampler,
            buckets: buckets.into_iter().collect(),
            bucket_type: PhantomData,
        }
    }

    /// Selects up to n elements.
    pub(crate) fn select(self, n: usize) -> Result<Vec<T>, Error>
    where
        BS: BetaSample,
        B: Bucket<T>,
    {
        self.take(n).collect()
    }
}

impl<'b, BS, B, T> Iterator for SelectionIter<'b, BS, B, T>
where
    BS: BetaSample,
    B: Bucket<T>,
{
    type Item = Result<T, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        pull_arms(&self.beta_sampler, self.buckets.iter_mut()).map(|result| {
            result.map(|bucket| bucket.pop().unwrap(/* sampled bucket is not empty */))
        })
    }
}

#[cfg(test)]
mod tests {
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

        fn retain_newest(&mut self, keep: usize) {
            let len = self.docs.len();
            if len > keep {
                self.docs.drain(..len - keep);
            }
        }
    }

    #[test]
    fn test_selection() {
        let mut stack_0 = Stack {
            alpha: 0.01,
            beta: 1.0,
            docs: vec![],
        };
        let mut stack_1 = Stack {
            alpha: 0.01,
            beta: 1.0,
            docs: vec![0],
        };
        let mut stack_2 = Stack {
            alpha: 1.0,
            beta: 0.001,
            docs: vec![1, 2, 3],
        };
        let mut stack_3 = Stack {
            alpha: 0.001,
            beta: 1.0,
            docs: vec![4, 5, 6],
        };

        let stacks = vec![&mut stack_0, &mut stack_1, &mut stack_2, &mut stack_3];

        let docs = SelectionIter::new(MockBetaSampler, stacks)
            .select(10)
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
