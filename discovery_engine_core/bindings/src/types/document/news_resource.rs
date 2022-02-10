use core::document::NewsResource;
use std::ptr::addr_of_mut;

use chrono::NaiveDateTime;
use url::Url;

/// Returns a pointer to the `title` field of a news resource.
///
/// # Safety
///
/// The pointer must point to a valid [`NewsResource`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn news_resource_place_of_title(place: *mut NewsResource) -> *mut String {
    unsafe { addr_of_mut!((*place).title) }
}

/// Returns a pointer to the `snippet` field of a news resource.
///
/// # Safety
///
/// The pointer must point to a valid [`NewsResource`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn news_resource_place_of_snippet(place: *mut NewsResource) -> *mut String {
    unsafe { addr_of_mut!((*place).snippet) }
}

/// Returns a pointer to the `url` field of a news resource.
///
/// # Safety
///
/// The pointer must point to a valid [`NewsResource`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn news_resource_place_of_url(place: *mut NewsResource) -> *mut Url {
    unsafe { addr_of_mut!((*place).url) }
}

/// Returns a pointer to the `source_url` field of a news resource.
///
/// # Safety
///
/// The pointer must point to a valid [`NewsResource`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn news_resource_place_of_source_url(place: *mut NewsResource) -> *mut Url {
    unsafe { addr_of_mut!((*place).source_url) }
}

/// Returns a pointer to the `date_publ` field of a news resource.
///
/// # Safety
///
/// The pointer must point to a valid [`NewsResource`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn news_resource_place_of_date_published(
    place: *mut NewsResource,
) -> *mut NaiveDateTime {
    unsafe { addr_of_mut!((*place).date_published) }
}

/// Returns a pointer to the `thumbnail` field of a news resource.
///
/// # Safety
///
/// The pointer must point to a valid [`NewsResource`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn news_resource_place_of_thumbnail(
    place: *mut NewsResource,
) -> *mut Option<Url> {
    unsafe { addr_of_mut!((*place).thumbnail) }
}

/// Returns a pointer to the `rank` field of a news resource.
///
/// # Safety
///
/// The pointer must point to a valid [`NewsResource`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn news_resource_place_of_rank(place: *mut NewsResource) -> *mut usize {
    unsafe { addr_of_mut!((*place).rank) }
}

/// Returns a pointer to the `score` field of a news resource.
///
/// # Safety
///
/// The pointer must point to a valid [`NewsResource`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn news_resource_place_of_score(
    place: *mut NewsResource,
) -> *mut Option<f32> {
    unsafe { addr_of_mut!((*place).score) }
}

/// Returns a pointer to the `country` field of a news resource.
///
/// # Safety
///
/// The pointer must point to a valid [`NewsResource`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn news_resource_place_of_country(place: *mut NewsResource) -> *mut String {
    unsafe { addr_of_mut!((*place).country) }
}

/// Returns a pointer to the `language` field of a news resource.
///
/// # Safety
///
/// The pointer must point to a valid [`NewsResource`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn news_resource_place_of_language(place: *mut NewsResource) -> *mut String {
    unsafe { addr_of_mut!((*place).language) }
}

/// Returns a pointer to the `topic` field of a news resource.
///
/// # Safety
///
/// The pointer must point to a valid [`NewsResource`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn news_resource_place_of_topic(place: *mut NewsResource) -> *mut String {
    unsafe { addr_of_mut!((*place).topic) }
}

/// Alloc an uninitialized `Box<NewsResource>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_news_resource() -> *mut NewsResource {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<NewsResource>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Document>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_news_resource(document: *mut NewsResource) {
    unsafe { crate::types::boxed::drop(document) };
}
