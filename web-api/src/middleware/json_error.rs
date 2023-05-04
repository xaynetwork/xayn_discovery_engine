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
use std::{fmt::Debug, future::Future};

use actix_web::{
    body::{BoxBody, MessageBody},
    dev::{Service, ServiceRequest, ServiceResponse},
    http::{header::CONTENT_TYPE, StatusCode},
    HttpResponse,
    ResponseError,
};
use derive_more::Display;
use futures_util::{
    future::{self, Either},
    TryFutureExt,
};
use serde_json::{json, Value};
use tracing::Level;

use super::request_context::{RequestContext, RequestId};
use crate::error::{early_failure::middleware_failure, json_error::JsonErrorResponseBuilder};

pub(crate) fn wrap_non_json_errors<S, B>(
    request: ServiceRequest,
    service: &S,
) -> impl Future<Output = Result<ServiceResponse<BoxBody>, actix_web::Error>> + 'static
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: MessageBody + Debug + 'static,
{
    let request_id = match RequestContext::try_extract_from_request(request.request(), |context| {
        context.request_id
    }) {
        Ok(id) => id,
        Err(error) => {
            let response = middleware_failure(
                "wrap_non_json_errors",
                request,
                None,
                None,
                error,
                Level::ERROR,
            );
            return Either::Left(future::ok(response));
        }
    };

    Either::Right(
        service
            .call(request)
            .map_ok(move |resp| wrap_service_response(resp, request_id))
            // note that endpoints _directly_ turn any `Err(..)` into an `Ok(err_resp)`,
            // so we will only see middleware errors here, never endpoint errors
            .map_err(move |resp| WrappedMiddlewareError::wrap(resp, request_id)),
    )
}

fn wrap_service_response<B: MessageBody + Debug + 'static>(
    response: ServiceResponse<B>,
    request_id: RequestId,
) -> ServiceResponse<BoxBody> {
    if is_wrappable_error(response.response()) {
        let (request, response) = response.into_parts();
        let (response, body) = response.into_parts();
        let details = extract_message_as_details(body);
        let response =
            JsonErrorResponseBuilder::render(response.status().as_str(), request_id, &details)
                .apply_to(response);
        ServiceResponse::new(request, response)
    } else {
        response.map_into_boxed_body()
    }
}

fn is_wrappable_error<B>(response: &HttpResponse<B>) -> bool {
    let status = response.status();
    (status.is_client_error() || status.is_server_error())
        && response
            .headers()
            .get(CONTENT_TYPE)
            .map_or(true, |content_type| {
                let mime = mime::TEXT_PLAIN.as_ref().as_bytes();
                let bytes = content_type.as_ref();
                bytes == mime || (bytes.starts_with(mime) && bytes.get(mime.len()) == Some(&b';'))
            })
}

fn extract_message_as_details(body: impl MessageBody + Debug) -> Value {
    let opt_bytes = body.try_into_bytes().ok();
    opt_bytes
        .as_deref()
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .filter(|msg| !msg.is_empty())
        .map_or(Value::Null, |msg| json!({ "message": msg }))
}

#[derive(Debug, Display)]
#[display(fmt = "{error}")]
pub(super) struct WrappedMiddlewareError {
    pub(super) error: actix_web::Error,
    pub(super) request_id: RequestId,
}

impl WrappedMiddlewareError {
    fn wrap(error: actix_web::Error, request_id: RequestId) -> actix_web::Error {
        Self { error, request_id }.into()
    }
}

impl ResponseError for WrappedMiddlewareError {
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
