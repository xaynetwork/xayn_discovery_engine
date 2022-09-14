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

use chrono::{DateTime, TimeZone, Utc};

/// cbindgen:ignore
pub(super) type DateTimeUtc = DateTime<Utc>;

const NANOS_PER_MICRO: i64 = 1_000;
const MICROS_PER_SECOND: i64 = 1_000_000;

/// [`DateTimeUtc::MAX_UTC`] in micros
const MAX_MICRO_SECONDS: i64 = 8_210_298_412_799_999_999;
/// [`DateTimeUtc::MIN_UTC`] in micros
const MIN_MICRO_SECONDS: i64 = -8_334_632_851_200_000_000;

/// Creates a rust `DateTimeUtc` at given memory address.
///
/// A time above the max or below the min supported
/// date time will be clamped to the max/min time.
///
/// # Safety
///
/// It must be valid to write a `DateTimeUtc` instance to given pointer,
/// the pointer is expected to point to uninitialized memory.
#[no_mangle]
pub unsafe extern "C" fn init_date_time_utc_at(place: *mut DateTimeUtc, micros_since_epoch: i64) {
    let date_time_utc = create_date_time_utc(micros_since_epoch);
    unsafe {
        place.write(date_time_utc);
    }
}

/// Initializes a rust `Option<DateTimeUtc>` at given memory address.
///
/// If  `i64::MIN` is passed in it will be initialized to `None`,
/// else to `Some`.
///
/// A time above the max or below the min supported
/// date time will be clamped to the max/min time.
///
/// # Safety
///
/// It must be valid to write a `DateTimeUtc` instance to given pointer,
/// the pointer is expected to point to uninitialized memory.
#[no_mangle]
pub unsafe extern "C" fn init_option_date_time_utc_at(
    place: *mut Option<DateTimeUtc>,
    micros_since_epoch: i64,
) {
    let value = (micros_since_epoch != i64::MIN).then(|| create_date_time_utc(micros_since_epoch));
    unsafe {
        place.write(value);
    }
}

fn create_date_time_utc(micros_since_epoch: i64) -> DateTimeUtc {
    let micros_since_epoch = micros_since_epoch.clamp(MIN_MICRO_SECONDS, MAX_MICRO_SECONDS);
    let seconds = micros_since_epoch / MICROS_PER_SECOND;
    let nanos = (micros_since_epoch.abs() % MICROS_PER_SECOND) * NANOS_PER_MICRO;
    // the unwraps failing is unreachable, but we do not want to have any panic path
    let nanos = u32::try_from(nanos).unwrap_or(u32::MAX);
    Utc.timestamp_opt(seconds, nanos)
        .single()
        .unwrap_or(DateTimeUtc::MAX_UTC)
}

/// Returns the number of micro seconds since midnight on January 1, 1970.
///
/// More specifically it's the number of micro seconds since `1970-01-01T00:00:00Z`.
#[no_mangle]
pub extern "C" fn get_date_time_utc_micros_since_epoch(date_time: &DateTimeUtc) -> i64 {
    let sub_micros = date_time.timestamp_subsec_micros();
    let seconds = date_time.timestamp();
    seconds * MICROS_PER_SECOND + i64::from(sub_micros)
}

/// Returns the number of micro seconds since midnight on January 1, 1970.
///
/// More specifically it's the number of micro seconds since `1970-01-01T00:00:00Z`.
///
/// If the `Option<DateTimeUtc>` is `None` then `i64::MIN` is returned instead.
///
/// As `i64::MIN < MIN_MICRO_SECONDS` we can differentiate it from the smallest date time.
#[no_mangle]
pub extern "C" fn get_option_date_time_utc_micros_since_epoch(
    date_time: &Option<DateTimeUtc>,
) -> i64 {
    if let Some(date_time) = date_time {
        get_date_time_utc_micros_since_epoch(date_time)
    } else {
        i64::MIN
    }
}

/// Alloc an uninitialized `Box<DateTimeUtc>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_date_time_utc() -> *mut DateTimeUtc {
    super::boxed::alloc_uninitialized()
}

/// Drops a `Box<DateTimeUtc>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent an initialized `Box<DateTimeUtc>`.
#[no_mangle]
pub unsafe extern "C" fn drop_date_time_utc(date_time_utc: *mut DateTimeUtc) {
    unsafe { super::boxed::drop(date_time_utc) }
}

#[cfg(test)]
mod tests {
    use std::mem::MaybeUninit;

    use chrono::Timelike;

    use super::*;

    #[test]
    fn test_max_date_is_supported() {
        let mut place = MaybeUninit::uninit();
        unsafe { init_date_time_utc_at(place.as_mut_ptr(), MAX_MICRO_SECONDS) };
        let place = unsafe { place.assume_init() };
        let truncated_max = DateTimeUtc::MAX_UTC.with_nanosecond(999_999_000).unwrap();
        assert_eq!(place, truncated_max);

        let mut place = MaybeUninit::uninit();
        let micros = get_date_time_utc_micros_since_epoch(&DateTimeUtc::MAX_UTC);
        unsafe { init_date_time_utc_at(place.as_mut_ptr(), micros) };
        let place = unsafe { place.assume_init() };
        assert_eq!(place, truncated_max);
    }

    #[test]
    fn test_min_date_is_supported() {
        let mut place = MaybeUninit::uninit();
        unsafe { init_date_time_utc_at(place.as_mut_ptr(), MIN_MICRO_SECONDS) };
        let place = unsafe { place.assume_init() };
        assert_eq!(place, DateTimeUtc::MIN_UTC);

        let mut place = MaybeUninit::uninit();
        let micros = get_date_time_utc_micros_since_epoch(&DateTimeUtc::MIN_UTC);
        unsafe { init_date_time_utc_at(place.as_mut_ptr(), micros) };
        let place = unsafe { place.assume_init() };
        assert_eq!(place, DateTimeUtc::MIN_UTC);
    }

    #[test]
    fn test_consts_max_is_sync() {
        let seconds = DateTimeUtc::MAX_UTC.timestamp();
        let sub_micros = DateTimeUtc::MAX_UTC.timestamp_subsec_micros();
        let micros = seconds.checked_mul(MICROS_PER_SECOND).unwrap() + i64::from(sub_micros);
        assert_eq!(micros, MAX_MICRO_SECONDS);
    }

    #[test]
    fn test_consts_min_is_sync() {
        let seconds = DateTimeUtc::MIN_UTC.timestamp();
        let sub_micros = DateTimeUtc::MIN_UTC.timestamp_subsec_micros();
        let micros = seconds.checked_mul(MICROS_PER_SECOND).unwrap() + i64::from(sub_micros);
        assert_eq!(micros, MIN_MICRO_SECONDS);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_none_marker_is_usable() {
        assert!(i64::MIN < MIN_MICRO_SECONDS);
    }

    #[test]
    fn test_init_option_date_time() {
        let mut place = MaybeUninit::uninit();
        unsafe { init_option_date_time_utc_at(place.as_mut_ptr(), 123) };
        let place = unsafe { place.assume_init() };
        assert_eq!(place, Some(Utc.timestamp(0, 123_000)));

        let mut place = MaybeUninit::uninit();
        unsafe { init_option_date_time_utc_at(place.as_mut_ptr(), i64::MIN) };
        let place = unsafe { place.assume_init() };
        assert_eq!(place, None);
    }

    #[test]
    fn test_read_option_date_time() {
        let date_time = Some(Utc.timestamp(1, 123_000));
        assert_eq!(
            get_option_date_time_utc_micros_since_epoch(&date_time),
            1_000_123
        );
        let date_time = None;
        assert_eq!(
            get_option_date_time_utc_micros_since_epoch(&date_time),
            i64::MIN
        );
    }
}
