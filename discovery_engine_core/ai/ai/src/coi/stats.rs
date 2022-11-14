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
    utils::{system_time_now, SECONDS_PER_DAY_F32},
};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct CoiStats {
    pub view_count: usize,
    pub view_time: Duration,
    pub last_view: SystemTime,
}

impl CoiStats {
    pub(super) fn new() -> Self {
        Self {
            view_count: 1,
            view_time: Duration::ZERO,
            last_view: system_time_now(),
        }
    }

    pub(super) fn log_time(&mut self, viewed: Duration) {
        self.view_count += 1;
        self.view_time += viewed;
    }

    pub(super) fn log_reaction(&mut self) {
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
    pub(crate) fn log_time(&mut self, viewed: Duration) -> &mut Self {
        self.stats.log_time(viewed);
        self
    }

    pub(super) fn log_reaction(&mut self) -> &mut Self {
        self.stats.log_reaction();
        self
    }
}

impl NegativeCoi {
    pub(super) fn log_reaction(&mut self) -> &mut Self {
        self.last_view = system_time_now();
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
    now: SystemTime,
) -> Vec<f32> {
    #[allow(clippy::cast_precision_loss)] // small values
    let counts = cois.iter().map(|coi| coi.stats.view_count).sum::<usize>() as f32 + f32::EPSILON;
    let times = cois
        .iter()
        .map(|coi| coi.stats.view_time)
        .sum::<Duration>()
        .as_secs_f32()
        + f32::EPSILON;

    cois.iter()
        .map(|coi| {
            #[allow(clippy::cast_precision_loss)] // small values
            let count = coi.stats.view_count as f32 / counts;
            let time = coi.stats.view_time.as_secs_f32() / times;
            let last = compute_coi_decay_factor(horizon, now, coi.stats.last_view);

            ((count + time) * last).max(0.).min(f32::MAX)
        })
        .collect()
}

/// Computes the time decay factor for a coi based on its `last_view` stat.
pub(super) fn compute_coi_decay_factor(
    horizon: Duration,
    now: SystemTime,
    last_view: SystemTime,
) -> f32 {
    const DAYS_SCALE: f32 = -0.1;
    let horizon = (horizon.as_secs_f32() * DAYS_SCALE / SECONDS_PER_DAY_F32).exp();
    let days = (now
        .duration_since(last_view)
        .unwrap_or_default()
        .as_secs_f32()
        * DAYS_SCALE
        / SECONDS_PER_DAY_F32)
        .exp();

    ((horizon - days) / (horizon - 1. - f32::EPSILON)).max(0.)
}

#[cfg(test)]
mod tests {
    use xayn_discovery_engine_test_utils::assert_approx_eq;

    use super::*;
    use crate::coi::{config::Config, point::tests::create_pos_cois};

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
        let config = Config::default().with_horizon(Duration::from_secs_f32(SECONDS_PER_DAY_F32));

        let relevances = compute_coi_relevances(&cois, config.horizon(), system_time_now());
        assert_approx_eq!(
            f32,
            relevances,
            [0.166_666_46, 0.333_332_93, 0.499_999_37],
            epsilon = 1e-6,
        );
    }

    #[test]
    fn test_compute_relevances_time() {
        let mut cois = create_pos_cois(&[[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]]);
        cois[1].stats.view_time += Duration::from_secs(10);
        cois[2].stats.view_time += Duration::from_secs(20);
        let config = Config::default().with_horizon(Duration::from_secs_f32(SECONDS_PER_DAY_F32));

        let relevances = compute_coi_relevances(&cois, config.horizon(), system_time_now());
        assert_approx_eq!(
            f32,
            relevances,
            [0.333_332_93, 0.666_666_7, 0.999_998_75],
            epsilon = 1e-6,
        );
    }

    #[test]
    fn test_compute_relevances_last() {
        let mut cois = create_pos_cois(&[[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]]);
        cois[0].stats.last_view -= Duration::from_secs_f32(0.5 * SECONDS_PER_DAY_F32);
        cois[1].stats.last_view -= Duration::from_secs_f32(1.5 * SECONDS_PER_DAY_F32);
        cois[2].stats.last_view -= Duration::from_secs_f32(2.5 * SECONDS_PER_DAY_F32);
        let config =
            Config::default().with_horizon(Duration::from_secs_f32(2. * SECONDS_PER_DAY_F32));

        let relevances = compute_coi_relevances(&cois, config.horizon(), system_time_now());
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

        let epoch = SystemTime::UNIX_EPOCH;
        let factor = compute_coi_decay_factor(horizon, epoch, epoch);
        assert_approx_eq!(f32, factor, 1.);

        let now = epoch + Duration::from_secs_f32(5. * SECONDS_PER_DAY_F32);
        let factor = compute_coi_decay_factor(horizon, now, epoch);
        assert_approx_eq!(f32, factor, 0.585_914_5);

        let now = epoch + Duration::from_secs_f32(30. * SECONDS_PER_DAY_F32);
        let factor = compute_coi_decay_factor(horizon, now, epoch);
        assert_approx_eq!(f32, factor, 0.);
    }
}
