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

use actix_web::{
    body::{BoxBody, MessageBody},
    http::{header::CONTENT_TYPE, StatusCode},
    HttpResponse,
    ResponseError,
};
use derive_more::Display;
use serde_json::{json, Value};
use uuid::Uuid;

use super::json_error::JsonErrorResponseBuilder;

#[derive(Debug, Display)]
#[display(fmt = "{}", error)]
pub(super) struct WrappedError {
    pub(super) error: actix_web::Error,
    pub(super) request_id: Uuid,
}

impl ResponseError for WrappedError {
    fn status_code(&self) -> StatusCode {
        self.error.as_response_error().status_code()
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let response = self.error.error_response();
        let wrap_in_json = response
            .headers()
            .get(CONTENT_TYPE)
            .map_or(false, |content_type| {
                content_type == mime::TEXT_PLAIN.as_ref()
            });

        if wrap_in_json {
            let (response, body) = response.into_parts();
            let opt_bytes = body.try_into_bytes().ok();
            let msg = opt_bytes
                .as_deref()
                .and_then(|bytes| std::str::from_utf8(bytes).ok());

            JsonErrorResponseBuilder::render(
                self.status_code().as_str(),
                self.request_id,
                &msg.map_or(Value::Null, |msg| json!({ "message": msg })),
            )
            .apply_to(response)
        } else {
            response
        }
    }
}
