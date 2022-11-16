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

use std::cmp::Ordering;

use displaydoc::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{kps::system::System, utils::nan_safe_f32_cmp_desc};

/// Configurations of the kps system.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[must_use]
pub struct Config {
    gamma: f32,
    penalty: Vec<f32>,
}

// the f32 fields are never NaN by construction
impl Eq for Config {}

impl Default for Config {
    fn default() -> Self {
        Self {
            gamma: 0.9,
            penalty: vec![1., 0.75, 0.66],
        }
    }
}

/// Errors of the kps system configuration.
#[derive(Copy, Clone, Debug, Display, Error)]
pub enum Error {
    /// Invalid coi gamma, expected value from the unit interval
    Gamma,
    /// Invalid coi penalty, expected non-empty, finite and sorted values
    Penalty,
}

impl Config {
    /// The weighting between coi and pairwise candidate similarities in the key phrase selection.
    pub fn gamma(&self) -> f32 {
        self.gamma
    }

    /// Sets the gamma.
    ///
    /// # Errors
    /// Fails if the gamma is outside of the unit interval.
    pub fn with_gamma(mut self, gamma: f32) -> Result<Self, Error> {
        if (0. ..=1.).contains(&gamma) {
            self.gamma = gamma;
            Ok(self)
        } else {
            Err(Error::Gamma)
        }
    }

    /// The penalty for less relevant key phrases of a coi in increasing order (ie. lowest penalty
    /// for the most relevant key phrase first and highest penalty for the least relevant key phrase
    /// last). The length of the penalty also serves as the maximum number of key phrases.
    pub fn penalty(&self) -> &[f32] {
        &self.penalty
    }

    /// Sets the penalty.
    ///
    /// # Errors
    /// Fails if the penalty is empty, has non-finite values or is unsorted.
    pub fn with_penalty(mut self, penalty: &[f32]) -> Result<Self, Error> {
        // TODO: refactor once slice::is_sorted_by() is stabilized
        fn is_sorted_by(slice: &[f32], compare: impl FnMut(&f32, &f32) -> Ordering) -> bool {
            let mut vector = slice.to_vec();
            vector.sort_unstable_by(compare);
            vector == slice
        }

        if !penalty.is_empty()
            && penalty.iter().copied().all(f32::is_finite)
            && is_sorted_by(penalty, nan_safe_f32_cmp_desc)
        {
            self.penalty = penalty.to_vec();
            Ok(self)
        } else {
            Err(Error::Penalty)
        }
    }

    /// The maximum number of key phrases picked during the coi key phrase selection.
    pub fn max_key_phrases(&self) -> usize {
        self.penalty.len()
    }

    /// Creates a kps system.
    pub fn build(self) -> System {
        System { config: self }
    }
}
