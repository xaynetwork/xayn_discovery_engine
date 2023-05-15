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

use std::time::Duration;

use displaydoc::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    system::System,
    utils::{serde_duration_as_days, SECONDS_PER_DAY},
};

/// Configurations of the coi system.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[must_use]
pub struct Config {
    shift_factor: f32,
    threshold: f32,
    min_positive_cois: usize,
    min_negative_cois: usize,
    #[serde(with = "serde_duration_as_days")]
    horizon: Duration,
}

// the f32 fields are never NaN by construction
impl Eq for Config {}

impl Default for Config {
    fn default() -> Self {
        Self {
            shift_factor: 0.1,
            threshold: 0.67,
            min_positive_cois: 1,
            min_negative_cois: 0,
            horizon: Duration::from_secs(30 * SECONDS_PER_DAY),
        }
    }
}

/// Errors of the coi system configuration.
#[derive(Copy, Clone, Debug, Display, Error)]
pub enum Error {
    /// Invalid coi shift factor, expected value from the unit interval
    ShiftFactor,
    /// Invalid coi threshold, expected non-negative value
    Threshold,
    /// Invalid minimum number of positive cois, expected positive value
    MinPositiveCois,
}

impl Config {
    /// The shift factor by how much a coi is shifted towards a new point.
    pub fn shift_factor(&self) -> f32 {
        self.shift_factor
    }

    /// Sets the shift factor.
    ///
    /// # Errors
    /// Fails if the shift factor is outside of the unit interval.
    pub fn with_shift_factor(mut self, shift_factor: f32) -> Result<Self, Error> {
        if (0. ..=1.).contains(&shift_factor) {
            self.shift_factor = shift_factor;
            Ok(self)
        } else {
            Err(Error::ShiftFactor)
        }
    }

    /// The maximum similarity between distinct cois.
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Sets the threshold.
    ///
    /// # Errors
    /// Fails if the threshold is not within [`-1, 1`].
    pub fn with_threshold(mut self, threshold: f32) -> Result<Self, Error> {
        if (-1. ..=1.).contains(&threshold) {
            self.threshold = threshold;
            Ok(self)
        } else {
            Err(Error::Threshold)
        }
    }

    /// The minimum number of positive cois required for the context calculation.
    pub fn min_positive_cois(&self) -> usize {
        self.min_positive_cois
    }

    /// Sets the minimum number of positive cois.
    ///
    /// # Errors
    /// Fails if the minimum number is zero.
    pub fn with_min_positive_cois(mut self, min_positive_cois: usize) -> Result<Self, Error> {
        if min_positive_cois > 0 {
            self.min_positive_cois = min_positive_cois;
            Ok(self)
        } else {
            Err(Error::MinPositiveCois)
        }
    }

    /// The minimum number of negative cois required for the context calculation.
    pub fn min_negative_cois(&self) -> usize {
        self.min_negative_cois
    }

    /// Sets the minimum number of negative cois.
    pub fn with_min_negative_cois(mut self, min_negative_cois: usize) -> Self {
        self.min_negative_cois = min_negative_cois;
        self
    }

    /// The time since the last view after which a coi becomes irrelevant.
    pub fn horizon(&self) -> Duration {
        self.horizon
    }

    /// Sets the horizon.
    pub fn with_horizon(mut self, horizon: Duration) -> Self {
        self.horizon = horizon;
        self
    }

    /// Creates a coi system.
    pub fn build(self) -> System {
        System { config: self }
    }
}
