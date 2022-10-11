use displaydoc::Display as DisplayDoc;
use thiserror::Error;

#[derive(Error, Debug, DisplayDoc)]
pub(crate) enum BackendError {
    /// Elastic search error: {0}
    Elastic(#[source] reqwest::Error),

    /// Error receiving response: {0}
    Receiving(#[source] reqwest::Error),
}

impl actix_web::error::ResponseError for BackendError {}
