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

use chrono::NaiveDateTime;

const NANOS_PER_MICRO: i64 = 1_000;
const MICROS_PER_SECOND: i64 = 1_000_000;

/// Creates a rust `NaiveDateTime` at given memory address.
///
/// Returns `1` if it succeeded `0` else wise.
///
/// # Safety
///
/// It must be valid to write a `NaiveDateTime` instance to given pointer,
/// the pointer is expected to point to uninitialized memory.
#[no_mangle]
pub unsafe extern "C" fn init_naive_date_time_at(
    place: *mut NaiveDateTime,
    micros_since_naive_epoch: i64,
) -> u8 {
    let seconds = micros_since_naive_epoch / MICROS_PER_SECOND;
    let nanos = (micros_since_naive_epoch.abs() % MICROS_PER_SECOND) * NANOS_PER_MICRO;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let nanos = nanos as u32;
    NaiveDateTime::from_timestamp_opt(seconds, nanos).map_or(0, |date_time| {
        unsafe { place.write(date_time) };
        1
    })
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
    naive_date_time: *mut NaiveDateTime,
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
