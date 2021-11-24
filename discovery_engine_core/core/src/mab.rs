use std::cmp::Ordering;

use displaydoc::Display;
use rand_distr::{Beta, BetaError, Distribution};
use thiserror::Error;

use crate::{engine::Stack, utils::nan_safe_f32_cmp, Document};

#[derive(Error, Debug, Display)]
pub(crate) enum MabError {
    /// Error while sampling
    Sampling(#[from] BetaError),
    /// No documents left in a stack
    EmptyStack,
    /// No stacks to pull from
    NoStacksToPull,
}

pub(crate) trait BetaSample {
    fn sample(&self, alpha: f32, beta: f32) -> Result<f32, MabError>;
}

/// Sample a value from a beta distribution
pub(crate) struct BetaSampler;

impl BetaSample for BetaSampler {
    fn sample(&self, alpha: f32, beta: f32) -> Result<f32, MabError> {
        Ok(Beta::new(alpha, beta)?.sample(&mut rand::thread_rng()))
    }
}

fn pull_arms(beta_sampler: &impl BetaSample, stacks: &mut [Stack]) -> Result<Document, MabError> {
    let sample_from_stack = |stack: &Stack| beta_sampler.sample(stack.alpha, stack.beta);

    let mut stacks = stacks.iter_mut();

    let first_stack = stacks.next().ok_or(MabError::NoStacksToPull)?;
    let first_sample = sample_from_stack(&first_stack)?;

    let stack = stacks
        .try_fold(
            (first_sample, first_stack),
            |max, stack| -> Result<_, MabError> {
                let sample = sample_from_stack(stack)?;
                if let Ordering::Greater = nan_safe_f32_cmp(&sample, &max.0) {
                    Ok((sample, stack))
                } else {
                    Ok(max)
                }
            },
        )?
        .1;

    stack.documents.pop().ok_or(MabError::EmptyStack)
}

pub(crate) struct MabSelection<BS> {
    beta_sampler: BS,
}

impl<BS> MabSelection<BS> {
    pub(crate) fn new(beta_sampler: BS) -> Self {
        Self { beta_sampler }
    }
}

impl<BS> MabSelection<BS>
where
    BS: BetaSample,
{
    pub(crate) fn select(&self, stacks: &mut [Stack], n: u32) -> Result<Vec<Document>, MabError> {
        (0..n)
            .map(|_| pull_arms(&self.beta_sampler, stacks))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use ndarray::{array, Ix1};

    use crate::{document::Embedding, DocumentId};

    use super::*;

    fn create_doc(id: u128) -> Document {
        Document {
            id: DocumentId::from_u128(id),
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
        let stack_1 = Stack::new(
            1.0,
            100.0,
            vec![create_doc(0), create_doc(1), create_doc(2)],
        );
        let stack_2 = Stack::new(20.0, 5.0, vec![create_doc(3), create_doc(4), create_doc(5)]);
        let stack_3 = Stack::new(
            1.0,
            1000.0,
            vec![create_doc(6), create_doc(7), create_doc(8)],
        );

        let mut stacks = vec![stack_1, stack_2, stack_3];
        let mab = MabSelection::new(BetaSampler);

        let mut docs = mab.select(&mut stacks, 3).unwrap();
        assert_eq!(docs.pop().unwrap().id, DocumentId::from_u128(3));
        assert_eq!(docs.pop().unwrap().id, DocumentId::from_u128(4));
        assert_eq!(docs.pop().unwrap().id, DocumentId::from_u128(5));
    }
}
