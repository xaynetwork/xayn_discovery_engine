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

use std::cmp::Ordering;

/// Pretend that f32 has a total ordering.
///
/// `NaN` is treated as the lowest possible value if `nan_min`, similar to what [`f32::max`] does.
/// Otherwise it is treated as the highest possible value, similar to what [`f32::min`] does.
#[inline]
#[allow(clippy::trivially_copy_pass_by_ref)] // required by calling functions
fn nan_safe_f32_cmp_base(a: &f32, b: &f32, nan_min: bool) -> Ordering {
    a.partial_cmp(b).unwrap_or_else(|| {
        // if `partial_cmp` returns None we have at least one `NaN`,
        let cmp = match (a.is_nan(), b.is_nan()) {
            (true, true) => Ordering::Equal,
            (true, _) => Ordering::Less,
            (_, true) => Ordering::Greater,
            _ => unreachable!("partial_cmp returned None but both numbers are not NaN"),
        };
        if nan_min {
            cmp
        } else {
            cmp.reverse()
        }
    })
}

/// Allows comparing and sorting f32 even if `NaN` is involved.
///
/// Pretend that f32 has a total ordering.
///
/// `NaN` is treated as the lowest possible value, similar to what [`f32::max`] does.
///
/// If this is used for sorting this will lead to an ascending order, like
/// for example `[NaN, 0.5, 1.5, 2.0]`.
///
/// By switching the input parameters around this can be used to create a
/// descending sorted order, like e.g.: `[2.0, 1.5, 0.5, NaN]`.
#[inline]
#[allow(clippy::trivially_copy_pass_by_ref)] // required by calling functions
pub fn nan_safe_f32_cmp(a: &f32, b: &f32) -> Ordering {
    nan_safe_f32_cmp_base(a, b, true)
}

/// `nan_safe_f32_cmp_desc(a,b)` is syntax sugar for `nan_safe_f32_cmp(b, a)`
#[inline]
#[allow(clippy::trivially_copy_pass_by_ref)] // required by calling functions
pub fn nan_safe_f32_cmp_desc(a: &f32, b: &f32) -> Ordering {
    nan_safe_f32_cmp(b, a)
}

/// The number of seconds per day (without leap seconds).
pub(crate) const SECONDS_PER_DAY_F32: f32 = 86400.;

/// The number of seconds per day (without leap seconds).
pub(crate) const SECONDS_PER_DAY_U64: u64 = 86400;

pub(crate) mod serde_duration_as_days {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::utils::SECONDS_PER_DAY_U64;

    pub(crate) fn serialize<S>(horizon: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (horizon.as_secs() / SECONDS_PER_DAY_U64).serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(deserializer).map(|days| Duration::from_secs(SECONDS_PER_DAY_U64 * days))
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error, time::Duration};

    use serde::{Deserialize, Serialize};
    use serde_json::{from_str, to_string};
    use xayn_ai_test_utils::assert_approx_eq;

    use super::*;

    #[test]
    fn test_nan_safe_f32_cmp_sorts_in_the_right_order() {
        let data = &mut [f32::NAN, 1., 5., f32::NAN, 4.];
        data.sort_by(nan_safe_f32_cmp);

        assert_approx_eq!(f32, &data[2..], [1., 4., 5.], ulps = 0);
        assert!(data[0].is_nan());
        assert!(data[1].is_nan());

        data.sort_by(nan_safe_f32_cmp_desc);

        assert_approx_eq!(f32, &data[..3], [5., 4., 1.], ulps = 0);
        assert!(data[3].is_nan());
        assert!(data[4].is_nan());

        let data = &mut [1., 5., 3., 4.];

        data.sort_by(nan_safe_f32_cmp);
        assert_approx_eq!(f32, &data[..], [1., 3., 4., 5.], ulps = 0);

        data.sort_by(nan_safe_f32_cmp_desc);
        assert_approx_eq!(f32, &data[..], [5., 4., 3., 1.], ulps = 0);
    }

    #[test]
    fn test_nan_safe_f32_cmp_nans_compare_as_expected() {
        assert_eq!(nan_safe_f32_cmp(&f32::NAN, &f32::NAN), Ordering::Equal);
        assert_eq!(nan_safe_f32_cmp(&-12., &f32::NAN), Ordering::Greater);
        assert_eq!(nan_safe_f32_cmp_desc(&-12., &f32::NAN), Ordering::Less);
        assert_eq!(nan_safe_f32_cmp(&f32::NAN, &-12.), Ordering::Less);
        assert_eq!(nan_safe_f32_cmp_desc(&f32::NAN, &-12.), Ordering::Greater);
        assert_eq!(nan_safe_f32_cmp(&12., &f32::NAN), Ordering::Greater);
        assert_eq!(nan_safe_f32_cmp_desc(&12., &f32::NAN), Ordering::Less);
        assert_eq!(nan_safe_f32_cmp(&f32::NAN, &12.), Ordering::Less);
        assert_eq!(nan_safe_f32_cmp_desc(&f32::NAN, &12.), Ordering::Greater);
    }

    #[derive(Deserialize, Serialize)]
    struct Days(#[serde(with = "serde_duration_as_days")] Duration);

    #[test]
    fn test_less() -> Result<(), Box<dyn Error>> {
        let duration = Duration::from_secs(SECONDS_PER_DAY_U64 - 1);
        let serialized = to_string(&Days(duration))?;
        assert_eq!(serialized, "0");
        let deserialized = from_str::<Days>(&serialized)?.0;
        assert_eq!(deserialized, Duration::ZERO);
        Ok(())
    }

    #[test]
    fn test_equal() -> Result<(), Box<dyn Error>> {
        let duration = Duration::from_secs(SECONDS_PER_DAY_U64);
        let serialized = to_string(&Days(duration))?;
        assert_eq!(serialized, "1");
        let deserialized = from_str::<Days>(&serialized)?.0;
        assert_eq!(deserialized, duration);
        Ok(())
    }

    #[test]
    fn test_greater() -> Result<(), Box<dyn Error>> {
        let duration = Duration::from_secs(SECONDS_PER_DAY_U64 + 1);
        let serialized = to_string(&Days(duration))?;
        assert_eq!(serialized, "1");
        let deserialized = from_str::<Days>(&serialized)?.0;
        assert_eq!(deserialized, Duration::from_secs(SECONDS_PER_DAY_U64));
        Ok(())
    }
}
