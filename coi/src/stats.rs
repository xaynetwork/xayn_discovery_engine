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
use serde::{Deserialize, Serialize};

use crate::{
    point::{NegativeCoi, PositiveCoi},
    utils::SECONDS_PER_DAY_F32,
};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct CoiStats {
    pub view_count: usize,
    pub view_time: Duration,
    pub last_view: DateTime<Utc>,
}

impl CoiStats {
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

impl PositiveCoi {
    pub fn log_time(&mut self, viewed: Duration) -> &mut Self {
        self.stats.log_time(viewed);
        self
    }

    pub(super) fn log_reaction(&mut self, time: DateTime<Utc>) -> &mut Self {
        self.stats.log_reaction(time);
        self
    }
}

impl NegativeCoi {
    pub(super) fn log_reaction(&mut self, time: DateTime<Utc>) -> &mut Self {
        self.last_view = time;
        self
    }
}

/// Computes the relevances of the positive cois.
///
/// The relevance of each coi is computed from its view count and view time relative to the
/// other cois. It's an unnormalized score from the interval `[0, ∞)`.
pub fn compute_coi_relevances(
    cois: &[PositiveCoi],
    horizon: Duration,
    time: DateTime<Utc>,
) -> Vec<f32> {
    #[allow(clippy::cast_precision_loss)] // small values
    let view_counts =
        cois.iter().map(|coi| coi.stats.view_count).sum::<usize>() as f32 + f32::EPSILON;
    let view_times = cois
        .iter()
        .map(|coi| coi.stats.view_time)
        .sum::<Duration>()
        .as_secs_f32()
        + f32::EPSILON;

    cois.iter()
        .map(|coi| {
            #[allow(clippy::cast_precision_loss)] // small values
            let view_count = coi.stats.view_count as f32 / view_counts;
            let view_time = coi.stats.view_time.as_secs_f32() / view_times;
            let decay = compute_coi_decay_factor(horizon, time, coi.stats.last_view);

            #[allow(clippy::manual_clamp)] // prevent NaN propagation
            ((view_count + view_time) * decay).max(0.).min(f32::MAX)
        })
        .collect()
}

/// Computes the time decay factor for a coi based on its `last_view` stat relative to the current
/// `time`.
pub fn compute_coi_decay_factor(
    horizon: Duration,
    time: DateTime<Utc>,
    last_view: DateTime<Utc>,
) -> f32 {
    const DAYS_SCALE: f32 = -0.1;
    let horizon = (horizon.as_secs_f32() * DAYS_SCALE / SECONDS_PER_DAY_F32).exp();
    let days = (time
        .signed_duration_since(last_view)
        .to_std()
        .unwrap_or_default()
        .as_secs_f32()
        * DAYS_SCALE
        / SECONDS_PER_DAY_F32)
        .exp();

    ((horizon - days) / (horizon - 1. - f32::EPSILON)).max(0.)
}

#[cfg(test)]
mod tests {
    use xayn_ai_test_utils::assert_approx_eq;

    use super::*;
    use crate::{config::Config, point::tests::create_pos_cois};

    #[test]
    fn test_compute_relevances_empty_cois() {
        let cois = Vec::new();
        let config = Config::default();

        let relevances = compute_coi_relevances(&cois, config.horizon(), Utc::now());
        assert!(relevances.is_empty());
    }

    #[test]
    fn test_compute_relevances_zero_horizon() {
        let cois = create_pos_cois([[1., 2., 3.], [4., 5., 6.]]);
        let config = Config::default().with_horizon(Duration::ZERO);

        let relevances = compute_coi_relevances(&cois, config.horizon(), Utc::now());
        assert_approx_eq!(f32, relevances, [0., 0.]);
    }

    #[test]
    fn test_compute_relevances_count() {
        let mut cois = create_pos_cois([[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]]);
        cois[1].stats.view_count += 1;
        cois[2].stats.view_count += 2;
        let config = Config::default().with_horizon(Duration::from_secs_f32(SECONDS_PER_DAY_F32));

        let relevances = compute_coi_relevances(&cois, config.horizon(), Utc::now());
        assert_approx_eq!(
            f32,
            relevances,
            [0.166_666_46, 0.333_332_93, 0.499_999_37],
            epsilon = 1e-6,
        );
    }

    #[test]
    fn test_compute_relevances_time() {
        let mut cois = create_pos_cois([[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]]);
        cois[1].stats.view_time += Duration::from_secs(10);
        cois[2].stats.view_time += Duration::from_secs(20);
        let config = Config::default().with_horizon(Duration::from_secs_f32(SECONDS_PER_DAY_F32));

        let relevances = compute_coi_relevances(&cois, config.horizon(), Utc::now());
        assert_approx_eq!(
            f32,
            relevances,
            [0.333_332_93, 0.666_666_7, 0.999_998_75],
            epsilon = 1e-6,
        );
    }

    #[test]
    fn test_compute_relevances_last() {
        let mut cois = create_pos_cois([[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]]);
        cois[0].stats.last_view -= chrono::Duration::hours(12);
        cois[1].stats.last_view -= chrono::Duration::hours(36);
        cois[2].stats.last_view -= chrono::Duration::hours(60);
        let config =
            Config::default().with_horizon(Duration::from_secs_f32(2. * SECONDS_PER_DAY_F32));

        let relevances = compute_coi_relevances(&cois, config.horizon(), Utc::now());
        assert_approx_eq!(
            f32,
            relevances,
            [0.243_649_72, 0.077_191_26, 0.],
            epsilon = 1e-6,
        );
    }

    #[test]
    fn test_compute_coi_decay_factor() {
        let horizon = Duration::from_secs_f32(30. * SECONDS_PER_DAY_F32);

        let now = Utc::now();
        let factor = compute_coi_decay_factor(horizon, now, now);
        assert_approx_eq!(f32, factor, 1.);

        let last = now - chrono::Duration::days(5);
        let factor = compute_coi_decay_factor(horizon, now, last);
        assert_approx_eq!(f32, factor, 0.585_914_55, epsilon = 1e-6);

        let last = now - chrono::Duration::from_std(horizon).unwrap();
        let factor = compute_coi_decay_factor(horizon, now, last);
        assert_approx_eq!(f32, factor, 0.);
    }
}
