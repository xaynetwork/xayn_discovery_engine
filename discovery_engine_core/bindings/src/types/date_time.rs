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
/// Returns `1` if it succeeded `0` else wise.
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
    let micros_since_epoch = micros_since_epoch.clamp(MIN_MICRO_SECONDS, MAX_MICRO_SECONDS);
    let seconds = micros_since_epoch / MICROS_PER_SECOND;
    let nanos = (micros_since_epoch.abs() % MICROS_PER_SECOND) * NANOS_PER_MICRO;
    // the unwraps failing is unreachable, but we do not want to have any panic path
    let nanos = u32::try_from(nanos).unwrap_or(u32::MAX);
    let date_time_utc = Utc
        .timestamp_opt(seconds, nanos)
        .single()
        .unwrap_or(DateTimeUtc::MAX_UTC);
    unsafe {
        place.write(date_time_utc);
    }
}

/// Returns the number of micro seconds since midnight on January 1, 1970.
///
/// More specifically it's the number of micro seconds since `1970-01-01T00:00:00Z`.
///
/// # Safety
///
/// The pointer must point to a soundly initialized `DateTimeUtc` instance.
#[no_mangle]
pub unsafe extern "C" fn get_date_time_utc_micros_since_epoch(
    date_time_utc: *const DateTimeUtc,
) -> i64 {
    let date_time_utc = unsafe { &*date_time_utc };
    let sub_micros = date_time_utc.timestamp_subsec_micros();
    let seconds = date_time_utc.timestamp();
    seconds * MICROS_PER_SECOND + i64::from(sub_micros)
}

/// Alloc an uninitialized `Box<DateTimeUtc>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_date_time_utc() -> *mut DateTimeUtc {
    super::boxed::alloc_uninitialized()
}

/// Drops a `Box<DateTimeUtc>`.
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
        let micros = unsafe { get_date_time_utc_micros_since_epoch(&DateTimeUtc::MAX_UTC) };
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
        let micros = unsafe { get_date_time_utc_micros_since_epoch(&DateTimeUtc::MIN_UTC) };
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
}
