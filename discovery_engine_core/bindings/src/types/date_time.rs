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

//! FFI functions for handling date time fields.
use std::convert::TryFrom;

use chrono::{naive::MAX_DATETIME, NaiveDateTime};

const NANOS_PER_MICRO: i64 = 1_000;
const MICROS_PER_SECOND: i64 = 1_000_000;

/// [`chrono::naive::MAX_DATETIME`] in micros
const MAX_MICRO_SECONDS: i64 = 8_210_298_412_799_999_999;
/// [`chrono::naive::MIN_DATETIME`] in micros
const MIN_MICRO_SECONDS: i64 = -8_334_632_851_200_000_000;

/// Creates a rust `NaiveDateTime` at given memory address.
///
/// Returns `1` if it succeeded `0` else wise.
///
/// A a time above the max or below the min supported
/// date time will be clamped to the max/min time.
///
/// # Safety
///
/// It must be valid to write a `NaiveDateTime` instance to given pointer,
/// the pointer is expected to point to uninitialized memory.
#[no_mangle]
pub unsafe extern "C" fn init_naive_date_time_at(
    place: *mut NaiveDateTime,
    micros_since_naive_epoch: i64,
) {
    let micros_since_naive_epoch =
        micros_since_naive_epoch.clamp(MIN_MICRO_SECONDS, MAX_MICRO_SECONDS);
    let seconds = micros_since_naive_epoch / MICROS_PER_SECOND;
    let nanos = (micros_since_naive_epoch.abs() % MICROS_PER_SECOND) * NANOS_PER_MICRO;
    // the unwraps failing is unreachable, but we do not want to have any panic path
    let nanos = u32::try_from(nanos).unwrap_or(u32::MAX);
    let date_time = NaiveDateTime::from_timestamp_opt(seconds, nanos).unwrap_or(MAX_DATETIME);
    unsafe {
        place.write(date_time);
    }
}

/// Returns the number of micro seconds since since midnight on January 1, 1970.
///
/// More specifically it's the number of micro seconds since `1970-01-01T00:00:00Z` assuming
/// the naive date time to be in UTC.
///
/// # Safety
///
/// The pointer must point to a sound initialized `NaiveDateTime` instance.
#[no_mangle]
pub unsafe extern "C" fn get_naive_date_time_micros_since_epoch(
    naive_date_time: *const NaiveDateTime,
) -> i64 {
    let naive_date_time = unsafe { &*naive_date_time };
    let sub_micros = naive_date_time.timestamp_subsec_micros();
    let seconds = naive_date_time.timestamp();
    seconds * MICROS_PER_SECOND + i64::from(sub_micros)
}

/// Alloc an uninitialized `Box<NaiveDateTime>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_naive_date_time() -> *mut NaiveDateTime {
    super::boxed::alloc_uninitialized()
}

/// Drops a `Box<NaiveDateTime>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent an initialized `Box<NaiveDateTime>`.
#[no_mangle]
pub unsafe extern "C" fn drop_naive_date_time(naive_date_time: *mut NaiveDateTime) {
    unsafe { super::boxed::drop(naive_date_time) }
}

#[cfg(test)]
mod tests {
    use chrono::{
        naive::{MAX_DATETIME, MIN_DATETIME},
        NaiveDate,
        Timelike,
    };

    use super::*;

    #[test]
    fn test_max_date_is_supported() {
        let place = &mut NaiveDate::from_ymd(1, 1, 1).and_hms(1, 1, 1);
        unsafe { init_naive_date_time_at(place, MAX_MICRO_SECONDS) };
        let truncated_max = MAX_DATETIME.with_nanosecond(999_999_000).unwrap();
        assert_eq!(*place, truncated_max);

        let micros = unsafe { get_naive_date_time_micros_since_epoch(&MAX_DATETIME) };
        unsafe { init_naive_date_time_at(place, micros) };
        assert_eq!(*place, truncated_max);
    }

    #[test]
    fn test_min_date_is_supported() {
        let place = &mut NaiveDate::from_ymd(1, 1, 1).and_hms(1, 1, 1);
        unsafe { init_naive_date_time_at(place, MIN_MICRO_SECONDS) };
        assert_eq!(*place, MIN_DATETIME);

        let micros = unsafe { get_naive_date_time_micros_since_epoch(&MIN_DATETIME) };
        unsafe { init_naive_date_time_at(place, micros) };
        assert_eq!(*place, MIN_DATETIME);
    }

    #[test]
    fn test_consts_max_is_sync() {
        let seconds = MAX_DATETIME.timestamp();
        let sub_micros = MAX_DATETIME.timestamp_subsec_micros();
        let micros = seconds.checked_mul(MICROS_PER_SECOND).unwrap() + i64::from(sub_micros);
        assert_eq!(micros, MAX_MICRO_SECONDS);
    }

    #[test]
    fn test_consts_min_is_sync() {
        let seconds = MIN_DATETIME.timestamp();
        let sub_micros = MIN_DATETIME.timestamp_subsec_micros();
        let micros = seconds.checked_mul(MICROS_PER_SECOND).unwrap() + i64::from(sub_micros);
        assert_eq!(micros, MIN_MICRO_SECONDS);
    }
}
