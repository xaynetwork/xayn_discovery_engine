use std::cmp::Ordering;

use displaydoc::Display;
use rand_distr::{Beta, BetaError, Distribution};
use thiserror::Error;

use crate::{engine::Stack, utils::nan_safe_f32_cmp, Document};

#[derive(Error, Debug, Display)]
pub(crate) enum Error {
    /// Error while sampling
    Sampling(#[from] BetaError),
    /// No documents left in a stack
    EmptyStack,
    /// No stacks to pull from
    NoStacksToPull,
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

fn pull_arms(beta_sampler: &impl BetaSample, stacks: &mut [&mut Stack]) -> Result<Document, Error> {
    let sample_from_stack = |stack: &Stack| beta_sampler.sample(stack.alpha, stack.beta);

    let mut stacks = stacks.iter_mut();

    let first_stack = stacks.next().ok_or(Error::NoStacksToPull)?;
    let first_sample = sample_from_stack(first_stack)?;

    let stack = stacks
        .try_fold(
            (first_sample, first_stack),
            |max, stack| -> Result<_, Error> {
                let sample = sample_from_stack(stack)?;
                if let Ordering::Greater = nan_safe_f32_cmp(&sample, &max.0) {
                    Ok((sample, stack))
                } else {
                    Ok(max)
                }
            },
        )?
        .1;

    stack.documents.pop().ok_or(Error::EmptyStack)
}

struct SelectionIter<'bs, 'stack, BS> {
    beta_sampler: &'bs BS,
    stacks: Vec<&'stack mut Stack>,
}

impl<'bs, 'stack, BS> SelectionIter<'bs, 'stack, BS> {
    fn new(beta_sampler: &'bs BS, stacks: Vec<&'stack mut Stack>) -> Self {
        Self {
            beta_sampler,
            stacks,
        }
    }
}

impl<'bs, 'stack, BS> Iterator for SelectionIter<'bs, 'stack, BS>
where
    BS: BetaSample,
{
    type Item = Result<Document, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut stack = vec![];
        std::mem::swap(&mut self.stacks, &mut stack);

        self.stacks = stack
            .into_iter()
            .filter(|stack| !stack.documents.is_empty())
            .collect::<Vec<&mut Stack>>();

        if self.stacks.is_empty() {
            None
        } else {
            Some(pull_arms(self.beta_sampler, &mut self.stacks))
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
    pub(crate) fn select(&self, stacks: Vec<&mut Stack>, n: u32) -> Result<Vec<Document>, Error> {
        let iter = SelectionIter::new(&self.beta_sampler, stacks);
        iter.take(n as usize).collect()
    }
}

#[cfg(test)]
mod tests {
    use ndarray::{array, Ix1};

    use crate::{document::Embedding, Id};

    use super::*;

    fn create_doc(id: u128) -> Document {
        Document {
            id: Id::from_u128(id),
            rank: 0,
            title: "title".into(),
            snippet: "snippet".into(),
            url: "url".into(),
            domain: "domain".into(),
            smbert_embedding: Embedding::<Ix1>(array![]),
        }
    }

    #[test]
    fn test_select() {
        let mut stack_1 = Stack::new(1.0, 100.0, vec![create_doc(0)]);
        let mut stack_2 = Stack::new(20.0, 5.0, vec![create_doc(3), create_doc(4), create_doc(5)]);
        let mut stack_3 = Stack::new(
            1.0,
            1000.0,
            vec![create_doc(6), create_doc(7), create_doc(8)],
        );

        let stacks = vec![&mut stack_1, &mut stack_2, &mut stack_3];
        let mab = Selection::new(BetaSampler);

        let docs = mab.select(stacks, 10).unwrap();
        assert_eq!(docs[0].id, Id::from_u128(5));
        assert_eq!(docs[1].id, Id::from_u128(4));
        assert_eq!(docs[2].id, Id::from_u128(3));
        assert_eq!(docs[3].id, Id::from_u128(0));
    }
}
