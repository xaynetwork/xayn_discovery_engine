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

/// Common data of a [`Stack`](super::Stack).
#[derive(Derivative, Deserialize, Serialize, Debug)]
#[derivative(Default)]
pub(crate) struct Data {
    /// The alpha parameter of the beta distribution.
    #[derivative(Default(value = "1."))]
    pub(super) alpha: f32,
    /// The beta parameter of the beta distribution.
    #[derivative(Default(value = "1."))]
    pub(super) beta: f32,
    /// Documents in the [`Stack`](super::Stack).
    pub(super) documents: Vec<Document>,
}

impl Data {
    #[allow(dead_code)]
    /// Create a `Data`.
    pub(crate) fn new(alpha: f32, beta: f32, documents: Vec<Document>) -> Result<Self, Error> {
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

    /// Retains only the top documents, given how many to keep.
    pub(crate) fn retain_top(&mut self, keep: usize) {
        let len = self.documents.len();
        if len > keep {
            self.documents.drain(..len - keep);
        }
    }
}

#[cfg(test)]
mod tests {
    use claim::{assert_err, assert_matches, assert_ok};

    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_stack_empty() {
        let stack = Data::default();

        assert_eq!(stack.alpha, 1.);
        assert_eq!(stack.beta, 1.);
        assert!(stack.documents.is_empty());
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_stack_from_parts() {
        let stack = Data::new(0. + f32::EPSILON, 0. + f32::EPSILON, vec![]);
        assert_ok!(stack);

        let stack = Data::new(0.0, 0.5, vec![]);
        assert_err!(&stack);
        assert_matches!(stack.unwrap_err(), Error::InvalidAlpha(x) if x == 0.0);

        let stack = Data::new(0.5, 0.0, vec![]);
        assert_err!(&stack);
        assert_matches!(stack.unwrap_err(), Error::InvalidBeta(x) if x == 0.0);

        let stack = Data::new(-0.0, 1.0, vec![]);
        assert_err!(&stack);
        assert_matches!(stack.unwrap_err(), Error::InvalidAlpha(x) if x == 0.0);

        let stack = Data::new(1.0, -0.0, vec![]);
        assert_err!(&stack);
        assert_matches!(stack.unwrap_err(), Error::InvalidBeta(x) if x == 0.0);

        let stack = Data::new(-1.0, 1.0, vec![]);
        assert_err!(&stack);
        assert_matches!(stack.unwrap_err(), Error::InvalidAlpha(x) if x == -1.0);

        let stack = Data::new(1.0, -1.0, vec![]);
        assert_err!(&stack);
        assert_matches!(stack.unwrap_err(), Error::InvalidBeta(x) if x == -1.0);
    }
}
