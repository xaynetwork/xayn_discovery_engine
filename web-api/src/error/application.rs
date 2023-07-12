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

use actix_web::{body::BoxBody, http::StatusCode, HttpResponse, ResponseError};
use derive_more::{Deref, Display};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use tracing::Level;

use super::json_error::JsonErrorResponseBuilder;
use crate::middleware::request_context::RequestId;

#[derive(Display, Debug, Deref, Error)]
#[deref(forward)]
pub struct Error {
    error: Box<dyn ApplicationError>,
}

impl Error {
    pub fn new(error: impl ApplicationError) -> Self {
        Self {
            error: Box::new(error),
        }
    }
}

impl<T> From<T> for Error
where
    T: ApplicationError,
{
    fn from(error: T) -> Self {
        Self::new(error)
    }
}

/// Workaround to use a non-constant `tracing::Level` in the `tracing::event!` macro.
macro_rules! application_event {
    ($level:expr, $($fields:tt)*) => {{
        match $level {
            ::tracing::Level::TRACE => ::tracing::trace!($($fields)*),
            ::tracing::Level::DEBUG => ::tracing::debug!($($fields)*),
            ::tracing::Level::INFO => ::tracing::info!($($fields)*),
            ::tracing::Level::WARN => ::tracing::warn!($($fields)*),
            ::tracing::Level::ERROR => ::tracing::error!($($fields)*),
        }
    }};
}
pub(crate) use application_event;

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        self.error.status_code()
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        // We log the event before rendering it as we likely will
        // not include all information in the response which we
        // might want to have in the logs.
        application_event!(self.level(), error=%self.error);
        let request_id =
            RequestId::extract_from_task_local_storage().unwrap_or(RequestId::missing());
        JsonErrorResponseBuilder::render(
            self.error.kind(),
            request_id,
            &self.error.encode_details(),
        )
        .into_response(self.error.status_code())
    }
}

pub trait ApplicationError: std::error::Error + Send + Sync + 'static {
    fn status_code(&self) -> StatusCode;
    fn kind(&self) -> &str;
    fn level(&self) -> Level;
    fn encode_details(&self) -> Value {
        Value::Null
    }
}

/// Implements `ApplicationError` for given type using given http status code.
macro_rules! impl_application_error {
    ($name:ident => $code:ident, $level:ident) => {
        impl $crate::error::application::ApplicationError for $name {
            fn status_code(&self) -> ::actix_web::http::StatusCode {
                ::actix_web::http::StatusCode::$code
            }

            fn kind(&self) -> &str {
                ::std::stringify!($name)
            }

            fn level(&self) -> ::tracing::Level {
                ::tracing::Level::$level
            }

            fn encode_details(&self) -> ::serde_json::Value {
                ::serde_json::to_value(self)
                    .unwrap_or_else(|error| {
                        ::tracing::error!(
                            %error,
                            "serializing error details failed",
                        );
                        ::serde_json::Value::Null
                    })
            }
        }
    };
}
pub(crate) use impl_application_error;

/// Given functionality is not implemented.
#[derive(Debug, Display, Error, Serialize)]
pub(crate) struct Unimplemented {
    pub(crate) functionality: &'static str,
}

impl_application_error!(Unimplemented => INTERNAL_SERVER_ERROR, ERROR);
