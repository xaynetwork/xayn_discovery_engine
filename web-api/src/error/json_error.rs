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
    http::{
        header::{TryIntoHeaderValue, CONTENT_TYPE},
        StatusCode,
    },
    HttpResponse,
};
use serde_json::{json, Value};
use tracing::error;

use crate::middleware::request_context::RequestId;

pub(crate) struct JsonErrorResponseBuilder {
    body: BoxBody,
}

impl JsonErrorResponseBuilder {
    pub(crate) fn internal_server_error(request_id: RequestId) -> HttpResponse {
        JsonErrorResponseBuilder::render("InternalServerError", request_id, &Value::Null)
            .into_response(StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub(crate) fn render(kind: &str, request_id: RequestId, details: &Value) -> Self {
        match serde_json::to_vec(&json!({
            "kind": kind,
            "request_id": request_id,
            "details": details
        })) {
            Ok(encoded) => Self {
                body: encoded.boxed(),
            },
            Err(error) => {
                error!("Failed to encode body: {error}");
                Self {
                    body: GENERIC_INTERNAL_SERVER_ERROR.boxed(),
                }
            }
        }
    }

    pub(crate) fn apply_to<B>(self, mut response: HttpResponse<B>) -> HttpResponse {
        response.headers_mut().insert(
            CONTENT_TYPE,
            //Unwrap: MIME is guaranteed well formed
            mime::APPLICATION_JSON.try_into_value().unwrap(),
        );
        response.set_body(self.body)
    }

    pub(crate) fn into_response(self, status_code: StatusCode) -> HttpResponse {
        HttpResponse::build(status_code)
            .content_type(mime::APPLICATION_JSON)
            .message_body(self.body)
            //Unwrap: MIME is guaranteed well formed
            .unwrap()
    }
}

const GENERIC_INTERNAL_SERVER_ERROR: &str = r#"{
    "kind": "Internal",
    "details": {}
}"#;
