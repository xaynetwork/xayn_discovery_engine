use chrono::{NaiveDateTime};

/// Creates a rust `NaiveDateTime` at given memory address.
///
/// # Safety
///
/// It must be valid to write a `NaiveDateTime` instance to given pointer,
/// the pointer is expected to point to uninitialized memory.
#[no_mangle]
pub unsafe extern "C" fn init_naive_date_time_at(
    place: *mut NaiveDateTime,
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    // rust millis or micros or nanos
    // dart millis part and micros part no nanos
) {

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
