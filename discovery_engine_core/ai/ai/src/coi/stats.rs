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

use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::{
    coi::point::{NegativeCoi, PositiveCoi},
    utils::{system_time_now, SECONDS_PER_DAY},
};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(crate) struct CoiStats {
    pub(crate) view_count: usize,
    pub(crate) view_time: Duration,
    pub(crate) last_view: SystemTime,
}

impl CoiStats {
    pub(crate) fn new() -> Self {
        Self {
            view_count: 1,
            view_time: Duration::ZERO,
            last_view: system_time_now(),
        }
    }

    pub(crate) fn log_time(&mut self, viewed: Duration) {
        self.view_count += 1;
        self.view_time += viewed;
    }

    pub(crate) fn log_reaction(&mut self) {
        self.view_count += 1;
        self.last_view = system_time_now();
    }
}

impl Default for CoiStats {
    fn default() -> Self {
        Self {
            view_count: 1,
            view_time: Duration::ZERO,
            last_view: SystemTime::UNIX_EPOCH,
        }
    }
}

impl PositiveCoi {
    pub(crate) fn log_time(&mut self, viewed: Duration) {
        self.stats.log_time(viewed);
    }

    pub(crate) fn log_reaction(&mut self) {
        self.stats.log_reaction();
    }
}

impl NegativeCoi {
    pub(crate) fn log_reaction(&mut self) {
        self.last_view = system_time_now();
    }
}

/// Computes the relevances of the positive cois.
///
/// The relevance of each coi is computed from its view count and view time relative to the
/// other cois. It's an unnormalized score from the interval `[0, ∞)`.
pub(crate) fn compute_coi_relevances(
    cois: &[PositiveCoi],
    horizon: Duration,
    now: SystemTime,
) -> Vec<f32> {
    let counts = cois.iter().map(|coi| coi.stats.view_count).sum::<usize>() as f32 + f32::EPSILON;
    let times = cois
        .iter()
        .map(|coi| coi.stats.view_time)
        .sum::<Duration>()
        .as_secs_f32()
        + f32::EPSILON;

    cois.iter()
        .map(|coi| {
            let count = coi.stats.view_count as f32 / counts;
            let time = coi.stats.view_time.as_secs_f32() / times;
            let last = compute_coi_decay_factor(horizon, now, coi.stats.last_view);

            ((count + time) * last).max(0.).min(f32::MAX)
        })
        .collect()
}

/// Computes the time decay factor for a coi based on its last_view stat.
pub(crate) fn compute_coi_decay_factor(
    horizon: Duration,
    now: SystemTime,
    last_view: SystemTime,
) -> f32 {
    const DAYS_SCALE: f32 = -0.1;
    let horizon = (horizon.as_secs_f32() * DAYS_SCALE / SECONDS_PER_DAY).exp();
    let days = (now
        .duration_since(last_view)
        .unwrap_or_default()
        .as_secs_f32()
        * DAYS_SCALE
        / SECONDS_PER_DAY)
        .exp();

    ((horizon - days) / (horizon - 1. - f32::EPSILON)).max(0.)
}

#[cfg(test)]
mod tests {
    use crate::coi::{config::Config, utils::tests::create_pos_cois};
    use xayn_discovery_engine_test_utils::assert_approx_eq;

    use super::*;

    #[test]
    fn test_compute_relevances_empty_cois() {
        let cois = create_pos_cois(&[[]]);
        let config = Config::default();

        let relevances = compute_coi_relevances(&cois, config.horizon(), system_time_now());
        assert!(relevances.is_empty());
    }

    #[test]
    fn test_compute_relevances_zero_horizon() {
        let cois = create_pos_cois(&[[1., 2., 3.], [4., 5., 6.]]);
        let config = Config::default().with_horizon(Duration::ZERO);

        let relevances = compute_coi_relevances(&cois, config.horizon(), system_time_now());
        assert_approx_eq!(f32, relevances, [0., 0.]);
    }

    #[test]
    fn test_compute_relevances_count() {
        let mut cois = create_pos_cois(&[[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]]);
        cois[1].stats.view_count += 1;
        cois[2].stats.view_count += 2;
        let config = Config::default().with_horizon(Duration::from_secs_f32(SECONDS_PER_DAY));

        let relevances = compute_coi_relevances(&cois, config.horizon(), system_time_now());
        assert_approx_eq!(
            f32,
            relevances,
            [0.16666646, 0.33333293, 0.49999937],
            epsilon = 1e-6,
        );
    }

    #[test]
    fn test_compute_relevances_time() {
        let mut cois = create_pos_cois(&[[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]]);
        cois[1].stats.view_time += Duration::from_secs(10);
        cois[2].stats.view_time += Duration::from_secs(20);
        let config = Config::default().with_horizon(Duration::from_secs_f32(SECONDS_PER_DAY));

        let relevances = compute_coi_relevances(&cois, config.horizon(), system_time_now());
        assert_approx_eq!(
            f32,
            relevances,
            [0.33333293, 0.6666667, 0.99999875],
            epsilon = 1e-6,
        );
    }

    #[test]
    fn test_compute_relevances_last() {
        let mut cois = create_pos_cois(&[[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]]);
        cois[0].stats.last_view -= Duration::from_secs_f32(0.5 * SECONDS_PER_DAY);
        cois[1].stats.last_view -= Duration::from_secs_f32(1.5 * SECONDS_PER_DAY);
        cois[2].stats.last_view -= Duration::from_secs_f32(2.5 * SECONDS_PER_DAY);
        let config = Config::default().with_horizon(Duration::from_secs_f32(2. * SECONDS_PER_DAY));

        let relevances = compute_coi_relevances(&cois, config.horizon(), system_time_now());
        assert_approx_eq!(
            f32,
            relevances,
            [0.24364972, 0.07719126, 0.],
            epsilon = 1e-6,
        );
    }

    #[test]
    fn test_compute_coi_decay_factor() {
        let horizon = Duration::from_secs_f32(30. * SECONDS_PER_DAY);

        let epoch = SystemTime::UNIX_EPOCH;
        let factor = compute_coi_decay_factor(horizon, epoch, epoch);
        assert_approx_eq!(f32, factor, 1.);

        let now = epoch + Duration::from_secs_f32(5. * SECONDS_PER_DAY);
        let factor = compute_coi_decay_factor(horizon, now, epoch);
        assert_approx_eq!(f32, factor, 0.5859145);

        let now = epoch + Duration::from_secs_f32(30. * SECONDS_PER_DAY);
        let factor = compute_coi_decay_factor(horizon, now, epoch);
        assert_approx_eq!(f32, factor, 0.);
    }
}
