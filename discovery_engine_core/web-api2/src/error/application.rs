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

use std::cell::Cell;

use actix_web::{body::BoxBody, http::StatusCode, HttpResponse, ResponseError};
use derive_more::{Deref, Display};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use tracing::error;
use uuid::Uuid;

use super::json_error::JsonErrorResponseBuilder;

#[derive(Display, Debug, Deref)]
#[display(fmt = "{}", error)]
pub struct Error {
    #[deref(forward)]
    error: Box<dyn ApplicationError>,
    request_id: Cell<Uuid>,
}

impl Error {
    #[must_use]
    pub(super) fn try_inject_request_id(this: &mut actix_web::Error, request_id: Uuid) -> bool {
        if let Some(this) = this.as_error::<Self>() {
            this.request_id.set(request_id);
            true
        } else {
            false
        }
    }
}

impl<T> From<T> for Error
where
    T: ApplicationError,
{
    fn from(error: T) -> Self {
        Self {
            error: Box::new(error),
            request_id: Cell::new(Uuid::nil()),
        }
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        self.error.status_code()
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        JsonErrorResponseBuilder::render(
            self.error.kind(),
            self.request_id.get(),
            &self.error.encode_details(),
        )
        .into_response(self.error.status_code())
    }
}

pub trait ApplicationError: std::error::Error + Send + Sync + 'static {
    fn status_code(&self) -> StatusCode;
    fn kind(&self) -> &str;
    fn encode_details(&self) -> Value {
        Value::Null
    }
}

/// Derives [`ApplicationError`] for given type using given http status code.
macro_rules! derive_application_error {
    ($name:ident => $code:ident) => {
        impl ApplicationError for $name {
            fn status_code(&self) -> StatusCode {
                StatusCode::$code
            }

            fn kind(&self) -> &str {
                stringify!($name)
            }

            fn encode_details(&self) -> Value {
                serde_json::to_value(self)
                    .unwrap_or_else(|err| {
                        error!(%err, "serializing error details failed");
                        Value::Null
                    })
            }
        }
    };
}

#[derive(Debug, Display, Error, Serialize)]
/// Given functionality is not implemented.
pub(crate) struct Unimplemented {
    pub(crate) functionality: &'static str,
}

derive_application_error!(Unimplemented => INTERNAL_SERVER_ERROR);
