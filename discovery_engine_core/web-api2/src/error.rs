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

// 1. client caused error
// 2. application error
// 3. bundle the id
// 4. make sure you have the id in handler

use std::borrow::Cow;

use actix_web::{
    body::{BoxBody, MessageBody},
    dev::{Service, ServiceRequest, ServiceResponse},
    http::{
        header::{TryIntoHeaderValue, CONTENT_TYPE},
        StatusCode,
    },
    HttpResponse,
    ResponseError,
};
use derive_more::{Deref, Display};
use mime::Mime;
use serde_json::{json, Value};
use tracing::{debug, error, info_span, Instrument};
use uuid::Uuid;

pub async fn json_error_bodies_middleware<S, B>(
    request: ServiceRequest,
    service: S,
) -> Result<ServiceResponse<B>, actix_web::Error>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
{
    //FIXME try to use tracing-actix-web and the request id of it
    let request_id = Uuid::new_v4();
    let span = info_span!(
        "request",
        path = %request.request().path(),
        method = %request.request().method(),
        request_id = %request_id,
    );

    debug!(parent: &span, "request received");

    let res = service
        .call(request)
        .instrument(span.clone())
        .await
        .map_err(|error| ErrorWithRequestId { error, request_id }.into());

    debug!(parent: &span, "request processed");

    res
}

#[derive(Display, Debug, Deref)]
#[display(fmt = "{}", error)]
#[deref(forward)]
pub struct Error {
    error: Box<dyn ApplicationError>,
}

impl<T> From<T> for Error
where
    T: ApplicationError,
{
    fn from(error: T) -> Self {
        Error {
            error: Box::new(error),
        }
    }
}

impl From<Box<dyn ApplicationError>> for Error {
    fn from(error: Box<dyn ApplicationError>) -> Self {
        Error { error }
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        self.error.status_code()
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        render_error_body(self.error.kind(), Uuid::nil(), &self.error.encode_details())
            .map(|(content_type, body)| {
                build_simple_response(self.error.status_code(), content_type, body)
            })
            .unwrap_or_else(|err_response| err_response)
    }
}

pub trait ApplicationError: std::error::Error + Send + Sync + 'static {
    fn status_code(&self) -> StatusCode;
    fn kind(&self) -> &str;
    fn encode_details(&self) -> Cow<'_, Value> {
        Cow::Owned(json!({}))
    }
}

#[derive(Debug, Display)]
#[display(fmt = "{}", error)]
struct ErrorWithRequestId {
    error: actix_web::Error,
    request_id: Uuid,
}

impl ResponseError for ErrorWithRequestId {
    fn status_code(&self) -> StatusCode {
        self.error.as_response_error().status_code()
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        if let Some(error) = self.error.as_error::<Error>() {
            render_error_body(error.kind(), self.request_id, &error.encode_details())
                .map(|(content_type, body)| {
                    build_simple_response(error.status_code(), content_type, body)
                })
                .unwrap_or_else(|err_response| err_response)
        } else {
            wrap_error_as_json_error(&self.error, self.request_id)
        }
    }
}

fn wrap_error_as_json_error(error: &actix_web::Error, request_id: Uuid) -> HttpResponse {
    let response = error.error_response();
    let wrap_in_json = response
        .headers()
        .get(CONTENT_TYPE)
        .map_or(false, |content_type| {
            content_type == mime::TEXT_PLAIN.as_ref()
        });

    if wrap_in_json {
        let (mut response, body) = response.into_parts();
        let opt_bytes = body.try_into_bytes().ok();
        let msg = opt_bytes
            .as_deref()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .unwrap_or("");

        render_error_body(
            "RoutingOrMiddlewareFailed",
            request_id,
            &json!({ "message": &msg }),
        )
        .map(move |(mime, body)| {
            response.headers_mut().insert(
                CONTENT_TYPE,
                mime.try_into_value().unwrap(/* MIME is guaranteed well formed */),
            );
            response.set_body(body)
        })
        .unwrap_or_else(|error_response| error_response)
    } else {
        response
    }
}

fn render_error_body(
    kind: &str,
    request_id: Uuid,
    details: &Value,
) -> Result<(Mime, BoxBody), HttpResponse> {
    match serde_json::to_vec(&json!({
        "kind": kind,
        "request_id": request_id,
        "details": details
    })) {
        Ok(encoded) => Ok((mime::APPLICATION_JSON, encoded.boxed())),
        Err(error) => {
            error!("Failed to encode body:  {}", error);
            Err(generic_internal_server_error())
        }
    }
}

const GENERIC_INTERNAL_SERVER_ERROR: &str = r#"{
    "kind": "Internal",
    "details": {}
}"#;

pub fn generic_internal_server_error() -> HttpResponse {
    build_simple_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        mime::APPLICATION_JSON,
        GENERIC_INTERNAL_SERVER_ERROR.boxed(),
    )
}

pub fn build_simple_response(
    status_code: StatusCode,
    content_type: Mime,
    body: BoxBody,
) -> HttpResponse {
    HttpResponse::build(status_code)
        .content_type(content_type)
        .message_body(body)
        .unwrap(/* can not fail as MIME is guaranteed well formed */)
}
