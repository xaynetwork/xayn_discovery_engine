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

use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::point::Coi;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Stats {
    pub view_count: usize,
    pub view_time: Duration,
    pub last_view: DateTime<Utc>,
}

impl Stats {
    pub(super) fn new(time: DateTime<Utc>) -> Self {
        Self {
            view_count: 1,
            view_time: Duration::ZERO,
            last_view: time,
        }
    }

    pub(super) fn log_time(&mut self, viewed: Duration) {
        self.view_time += viewed;
    }

    pub(super) fn log_reaction(&mut self, time: DateTime<Utc>) {
        self.view_count += 1;
        self.last_view = time;
    }
}

impl Coi {
    pub fn log_time(&mut self, viewed: Duration) -> &mut Self {
        self.stats.log_time(viewed);
        self
    }

    pub(super) fn log_reaction(&mut self, time: DateTime<Utc>) -> &mut Self {
        self.stats.log_reaction(time);
        self
    }
}

/// Computes the relevances of the [`Coi`]s.
///
/// The relevance of each coi is computed from its view count and view time relative to the
/// other cois and ranges in the interval `[0., 2.]`.
pub fn compute_coi_relevances<'a>(
    cois: impl IntoIterator<IntoIter = impl Clone + Iterator<Item = &'a Coi>>,
    horizon: Duration,
    time: DateTime<Utc>,
) -> Vec<f32> {
    let cois = cois.into_iter();

    let view_counts = cois.clone().map(|coi| coi.stats.view_count).sum::<usize>();
    #[allow(clippy::cast_precision_loss)]
    let view_counts = if view_counts == 0 {
        // arbitrary to allow for division since each view_count is zero
        1.
    } else {
        view_counts as f32
    };

    let view_times = cois
        .clone()
        .map(|coi| coi.stats.view_time)
        .sum::<Duration>();
    let view_times = if view_times == Duration::ZERO {
        // arbitrary to allow for division since each view_time is zero
        1.
    } else {
        view_times.as_secs_f32()
    };

    cois.map(|coi| {
        #[allow(clippy::cast_precision_loss)]
        let view_count = coi.stats.view_count as f32 / view_counts;
        let view_time = coi.stats.view_time.as_secs_f32() / view_times;
        let decay = compute_coi_decay_factor(horizon, time, coi.stats.last_view);

        (view_count + view_time) * decay
    })
    .collect()
}

/// Computes the time decay factor for a [`Coi`].
///
/// The decay factor is based on its `last_view` stat relative to the current `time` and ranges in
/// the interval `[0., 1.]`.
pub fn compute_coi_decay_factor(
    horizon: Duration,
    time: DateTime<Utc>,
    last_view: DateTime<Utc>,
) -> f32 {
    if horizon == Duration::ZERO {
        return 0.;
    }

    let Ok(days) = time.signed_duration_since(last_view).to_std() else {
        return 1.;
    };

    const DAYS_SCALE: f32 = -0.1 / (60. * 60. * 24.);
    let horizon = (DAYS_SCALE * horizon.as_secs_f32()).exp();
    let days = (DAYS_SCALE * days.as_secs_f32()).exp();

    ((horizon - days) / (horizon - 1.)).max(0.)
}

/// Computes a weight distributions across [`Coi`]s based on their relevance.
///
/// Each weight ranges in the interval `[0., 1.]`.
pub fn compute_coi_weights<'a>(
    cois: impl IntoIterator<IntoIter = impl Clone + Iterator<Item = &'a Coi>>,
    horizon: Duration,
    time: DateTime<Utc>,
) -> Vec<f32> {
    let relevances = compute_coi_relevances(cois, horizon, time)
        .into_iter()
        .map(|relevance| 1. - (-3. * relevance).exp())
        .collect_vec();
    let relevance_sum = relevances.iter().sum::<f32>();

    if relevance_sum > 0. {
        relevances
            .iter()
            .map(|relevance| relevance / relevance_sum)
            .collect()
    } else {
        #[allow(clippy::cast_precision_loss)] // should be ok for our use case
        let len = relevances.len().max(1) as f32;
        vec![1. / len; relevances.len()]
    }
}

#[cfg(test)]
mod tests {
    use xayn_test_utils::assert_approx_eq;

    use super::*;
    use crate::{point::tests::create_cois, utils::SECONDS_PER_DAY};

    #[test]
    fn test_compute_relevances_empty_cois() {
        let cois = [];
        let horizon = Duration::MAX;
        let now = Utc::now();

        let relevances = compute_coi_relevances(cois, horizon, now);
        assert!(relevances.is_empty());
    }

    #[test]
    fn test_compute_relevances_zero_horizon() {
        let now = Utc::now();
        let cois = create_cois([[1., 2., 3.], [4., 5., 6.]], now);
        let horizon = Duration::ZERO;

        let relevances = compute_coi_relevances(&cois, horizon, now);
        assert_approx_eq!(f32, relevances, [0., 0.]);
    }

    #[test]
    fn test_compute_relevances_count() {
        let now = Utc::now();
        let mut cois = create_cois([[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]], now);
        cois[1].stats.view_count += 1;
        cois[2].stats.view_count += 2;
        let horizon = Duration::from_secs(SECONDS_PER_DAY);

        let relevances = compute_coi_relevances(&cois, horizon, now);
        assert_approx_eq!(f32, relevances, [0.166_666_67, 0.333_333_34, 0.5]);
    }

    #[test]
    fn test_compute_relevances_time() {
        let now = Utc::now();
        let mut cois = create_cois([[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]], now);
        cois[1].stats.view_time += Duration::from_secs(10);
        cois[2].stats.view_time += Duration::from_secs(20);
        let horizon = Duration::from_secs(SECONDS_PER_DAY);

        let relevances = compute_coi_relevances(&cois, horizon, now);
        assert_approx_eq!(f32, relevances, [0.333_333_34, 0.666_666_7, 1.]);
    }

    #[test]
    fn test_compute_relevances_last() {
        let now = Utc::now();
        let mut cois = create_cois([[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]], now);
        cois[0].stats.last_view -= chrono::Duration::hours(12);
        cois[1].stats.last_view -= chrono::Duration::hours(36);
        cois[2].stats.last_view -= chrono::Duration::hours(60);
        let horizon = Duration::from_secs(2 * SECONDS_PER_DAY);

        let relevances = compute_coi_relevances(&cois, horizon, now);
        assert_approx_eq!(
            f32,
            relevances,
            [0.243_649_84, 0.077_191_29, 0.],
            epsilon = 1e-7,
        );
    }

    #[test]
    fn test_compute_coi_decay_factor() {
        let horizon = Duration::from_secs(30 * SECONDS_PER_DAY);

        let now = Utc::now();
        let factor = compute_coi_decay_factor(horizon, now, now);
        assert_approx_eq!(f32, factor, 1.);

        let last = now - chrono::Duration::days(5);
        let factor = compute_coi_decay_factor(horizon, now, last);
        assert_approx_eq!(f32, factor, 0.585_914_55);

        let last = now - chrono::Duration::from_std(horizon).unwrap();
        let factor = compute_coi_decay_factor(horizon, now, last);
        assert_approx_eq!(f32, factor, 0.);

        let factor = compute_coi_decay_factor(Duration::ZERO, now, now);
        assert_approx_eq!(f32, factor, 0.);
    }
}
