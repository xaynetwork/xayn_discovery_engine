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

//! FFI functions for handling `Duration` fields.

use std::time::Duration;

/// Initializes a [`Duration`] field at given place.
///
/// # Safety
///
/// It must be valid to write a [`Duration`] to given pointer.
#[no_mangle]
pub unsafe extern "C" fn init_duration_at(place: *mut Duration, seconds: u64, nanos: u32) {
    unsafe {
        place.write(Duration::new(seconds, nanos));
    }
}

/// Gets the seconds of a duration at given place.
///
/// # Safety
///
/// The pointer must point to a valid [`Duration`] instance.
#[no_mangle]
pub unsafe extern "C" fn get_duration_seconds(duration: *mut Duration) -> u64 {
    unsafe { &*duration }.as_secs()
}

/// Gets the (subseconds) nanoseconds of a duration at given place.
///
/// # Safety
///
/// The pointer must point to a valid [`Duration`] instance.
#[no_mangle]
pub unsafe extern "C" fn get_duration_nanos(duration: *mut Duration) -> u32 {
    unsafe { &*duration }.subsec_nanos()
}

/// Alloc an uninitialized `Box<Duration>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_duration() -> *mut Duration {
    super::boxed::alloc_uninitialized()
}

/// Drops a `Box<Duration>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Duration>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_duration(duration: *mut Duration) {
   unsafe {  super::boxed::drop(duration) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reading_duration() {
        let place = &mut Duration::new(78978, 89378);
        let read = unsafe {
            let secs = get_duration_seconds(place);
            let nanos = get_duration_nanos(place);
            Duration::new(secs, nanos)
        };
        assert_eq!(read, *place);
    }

    #[test]
    fn test_writing_duration() {
        let duration = Duration::new(78978, 89378);
        let place = &mut Duration::default();
        unsafe {
            init_duration_at(place, duration.as_secs(), duration.subsec_nanos());
        }
        assert_eq!(duration, *place);
    }
}
