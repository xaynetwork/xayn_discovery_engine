use derivative::Derivative;
use displaydoc::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::document::Document;

#[derive(Error, Debug, Display)]
#[allow(dead_code)]
pub(crate) enum Error {
    /// Invalid value for alpha: {0}. It must be greater than 0.
    InvalidAlpha(f32),
    /// Invalid value for beta: {0}. It must be greater than 0.
    InvalidBeta(f32),
}

/// Common data of a [`Stack`].
#[derive(Derivative, Deserialize, Serialize, Debug, Default)]
pub(crate) struct Data {
    /// The alpha parameter of the beta distribution.
    #[derivative(Default(value = "1."))]
    pub(super) alpha: f32,
    /// The beta parameter of the beta distribution.
    #[derivative(Default(value = "1."))]
    pub(super) beta: f32,
    /// Documents in the [`Data`].
    pub(super) documents: Vec<Document>,
}

impl Data {
    #[allow(dead_code)]
    /// Create a new `Data`.
    pub(crate) fn empty() -> Self {
        Self {
            alpha: 1.,
            beta: 1.,
            documents: vec![],
        }
    }

    #[allow(dead_code)]
    /// Create a `Data`.
    pub(crate) fn from_parts(
        alpha: f32,
        beta: f32,
        documents: Vec<Document>,
    ) -> Result<Self, Error> {
        if alpha <= 0.0 {
            return Err(Error::InvalidAlpha(alpha));
        }
        if beta <= 0.0 {
            return Err(Error::InvalidBeta(beta));
        }

        Ok(Self {
            alpha,
            beta,
            documents,
        })
    }
}

#[cfg(test)]
mod tests {
    use claim::{assert_err, assert_matches, assert_ok};

    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_stack_empty() {
        let stack = Data::empty();

        assert_eq!(stack.alpha, 1.);
        assert_eq!(stack.beta, 1.);
        assert!(stack.documents.is_empty());
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_stack_from_parts() {
        let stack = Data::from_parts(0. + f32::EPSILON, 0. + f32::EPSILON, vec![]);
        assert_ok!(stack);

        let stack = Data::from_parts(0.0, 0.5, vec![]);
        assert_err!(&stack);
        assert_matches!(stack.unwrap_err(), Error::InvalidAlpha(x) if x == 0.0);

        let stack = Data::from_parts(0.5, 0.0, vec![]);
        assert_err!(&stack);
        assert_matches!(stack.unwrap_err(), Error::InvalidBeta(x) if x == 0.0);

        let stack = Data::from_parts(-0.0, 1.0, vec![]);
        assert_err!(&stack);
        assert_matches!(stack.unwrap_err(), Error::InvalidAlpha(x) if x == 0.0);

        let stack = Data::from_parts(1.0, -0.0, vec![]);
        assert_err!(&stack);
        assert_matches!(stack.unwrap_err(), Error::InvalidBeta(x) if x == 0.0);

        let stack = Data::from_parts(-1.0, 1.0, vec![]);
        assert_err!(&stack);
        assert_matches!(stack.unwrap_err(), Error::InvalidAlpha(x) if x == -1.0);

        let stack = Data::from_parts(1.0, -1.0, vec![]);
        assert_err!(&stack);
        assert_matches!(stack.unwrap_err(), Error::InvalidBeta(x) if x == -1.0);
    }
}
