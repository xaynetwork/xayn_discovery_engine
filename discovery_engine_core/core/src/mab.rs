use std::{cmp::Ordering, marker::PhantomData};

use displaydoc::Display;
use rand_distr::{Beta, BetaError, Distribution};
use thiserror::Error;

use crate::utils::nan_safe_f32_cmp;

#[derive(Error, Debug, Display)]
pub(crate) enum Error {
    /// Error while sampling
    Sampling(#[from] BetaError),
    /// No items left in a [`Bucket`]
    EmptyBucket,
    /// No [`Bucket`] to pull from
    NoBucketsToPull,
}

pub(crate) trait BetaSample {
    fn sample(&self, alpha: f32, beta: f32) -> Result<f32, Error>;
}

/// Sample a value from a beta distribution
pub(crate) struct BetaSampler;

impl BetaSample for BetaSampler {
    fn sample(&self, alpha: f32, beta: f32) -> Result<f32, Error> {
        Ok(Beta::new(alpha, beta)?.sample(&mut rand::thread_rng()))
    }
}

pub(crate) trait Bucket<T> {
    /// The alpha parameter of the beta distribution.
    fn alpha(&self) -> f32;
    /// The beta parameter of the beta distribution.
    fn beta(&self) -> f32;
    /// Returns `true` if the bucket contains no elements.
    fn is_empty(&self) -> bool;
    /// Removes the last element from a bucket and returns it, or `None` if it is empty.
    fn pop(&mut self) -> Option<T>;
}

fn pull_arms<B, T>(beta_sampler: &impl BetaSample, buckets: &mut [&mut B]) -> Result<T, Error>
where
    B: Bucket<T>,
{
    let sample_from_bucket = |bucket: &B| beta_sampler.sample(bucket.alpha(), bucket.beta());

    let mut buckets = buckets.iter_mut();

    let first_bucket = buckets.next().ok_or(Error::NoBucketsToPull)?;
    let first_sample = sample_from_bucket(first_bucket)?;

    let bucket = buckets
        .try_fold(
            (first_sample, first_bucket),
            |max, bucket| -> Result<_, Error> {
                let sample = sample_from_bucket(bucket)?;
                if let Ordering::Greater = nan_safe_f32_cmp(&sample, &max.0) {
                    Ok((sample, bucket))
                } else {
                    Ok(max)
                }
            },
        )?
        .1;

    bucket.pop().ok_or(Error::EmptyBucket)
}

struct SelectionIter<'bs, 'b, BS, B, T>
where
    B: Bucket<T>,
{
    beta_sampler: &'bs BS,
    buckets: Vec<&'b mut B>,
    bucket_type: PhantomData<T>,
}

impl<'bs, 'b, BS, B, T> SelectionIter<'bs, 'b, BS, B, T>
where
    B: Bucket<T>,
{
    fn new(beta_sampler: &'bs BS, buckets: Vec<&'b mut B>) -> Self {
        Self {
            beta_sampler,
            buckets,
            bucket_type: PhantomData,
        }
    }
}

impl<'bs, 'b, BS, B, T> Iterator for SelectionIter<'bs, 'b, BS, B, T>
where
    BS: BetaSample,
    B: Bucket<T>,
{
    type Item = Result<T, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buckets = vec![];
        std::mem::swap(&mut self.buckets, &mut buckets);

        self.buckets = buckets
            .into_iter()
            .filter(|bucket| !bucket.is_empty())
            .collect::<Vec<&mut B>>();

        if self.buckets.is_empty() {
            None
        } else {
            Some(pull_arms(self.beta_sampler, &mut self.buckets))
        }
    }
}

pub(crate) struct Selection<BS> {
    beta_sampler: BS,
}

impl<BS> Selection<BS> {
    pub(crate) fn new(beta_sampler: BS) -> Self {
        Self { beta_sampler }
    }
}

impl<BS> Selection<BS>
where
    BS: BetaSample,
{
    pub(crate) fn select<B, T>(&self, buckets: Vec<&mut B>, n: u32) -> Result<Vec<T>, Error>
    where
        B: Bucket<T>,
    {
        let selection = SelectionIter::new(&self.beta_sampler, buckets);
        selection.take(n as usize).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mab = Selection::new(BetaSampler);

        let docs = mab.select(stacks, 10).unwrap();
        assert_eq!(docs[0], 3);
        assert_eq!(docs[1], 2);
        assert_eq!(docs[2], 1);
        assert_eq!(docs[3], 0);
        assert_eq!(docs[4], 6);
        assert_eq!(docs[5], 5);
        assert_eq!(docs[6], 4);
    }
}
