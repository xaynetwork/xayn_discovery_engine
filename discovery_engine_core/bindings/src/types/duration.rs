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

/// Initializes a [`Option<Duration>`] field at given place to `Some` value.
///
/// # Safety
///
/// It must be valid to write a [`Option<Duration>`] to given pointer.
#[no_mangle]
pub unsafe extern "C" fn init_option_duration_some_at(
    place: *mut Option<Duration>,
    seconds: u64,
    nanos: u32,
) {
    let value = Some(Duration::new(seconds, nanos));
    unsafe {
        place.write(value);
    }
}

/// Initializes a [`Option<Duration>`] field at given place to `None`.
///
/// # Safety
///
/// It must be valid to write a [`Option<Duration>`] to given pointer.
#[no_mangle]
pub unsafe extern "C" fn init_option_duration_none_at(place: *mut Option<Duration>) {
    unsafe {
        place.write(None);
    }
}

/// Gets the seconds of a duration at given place.
///
/// # Safety
///
/// The pointer must point to a valid [`Duration`] instance.
#[no_mangle]
pub extern "C" fn get_duration_seconds(duration: &Duration) -> u64 {
    duration.as_secs()
}

/// Returns true if given [`Option<Duration>`] is some.
///
/// Due to limitations of darts `ffigen` tool the `bool` is cast to `u8`.
#[no_mangle]
pub extern "C" fn get_option_duration_is_some(duration: &Option<Duration>) -> u8 {
    duration.is_some().into()
}

/// Gets the seconds of a optional duration at given place.
///
/// If the option is `None` the value `0` is returned, use
/// [`get_option_duration_is_some()`] to differentiate between a
/// `0`-duration and no duration.
#[no_mangle]
pub extern "C" fn get_option_duration_seconds(duration: &Option<Duration>) -> u64 {
    duration.as_ref().map_or(0, Duration::as_secs)
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

/// Gets the sub-second nano seconds of a optional duration at given place.
///
/// If the option is `None` the value `0` is returned, use
/// [`get_option_duration_is_some()`] to differentiate between a
/// `0`-duration and no duration.
#[no_mangle]
pub extern "C" fn get_option_duration_nanos(duration: &Option<Duration>) -> u32 {
    duration.as_ref().map_or(0, Duration::subsec_nanos)
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
    unsafe { super::boxed::drop(duration) };
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

    #[test]
    fn test_init_option_duration() {
        let duration = Duration::new(78978, 89378);
        //ok as Duration: !Drop
        let place = &mut None;
        unsafe {
            init_option_duration_some_at(place, duration.as_secs(), duration.subsec_nanos());
        }
        assert_eq!(*place, Some(duration));
        unsafe {
            init_option_duration_none_at(place);
        }
        assert_eq!(*place, None);
    }

    #[test]
    fn test_get_option_duration() {
        let place = &mut None;
        assert_eq!(get_option_duration_is_some(place), 0);
        assert_eq!(get_option_duration_seconds(place), 0);
        assert_eq!(get_option_duration_nanos(place), 0);
        *place = Some(Duration::new(78978, 89378));
        assert_eq!(get_option_duration_is_some(place), 1);
        assert_eq!(get_option_duration_seconds(place), 78978);
        assert_eq!(get_option_duration_nanos(place), 89378);
    }
}
