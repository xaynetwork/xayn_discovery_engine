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

use futures_util::FutureExt;
use serde::Serialize;
use std::future::Future;

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse},
    HttpMessage,
};

use tracing::{trace, Instrument};
use tracing_actix_web::root_span_macro::private::tracing::info_span;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, derive_more::Display, Serialize)]
#[serde(transparent)]
pub(crate) struct RequestId(Uuid);

impl RequestId {
    pub(crate) fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub(crate) const fn missing() -> Self {
        Self(Uuid::nil())
    }
}

pub(crate) fn tracing_log_request<S, B>(
    request: ServiceRequest,
    service: &S,
) -> impl Future<Output = Result<ServiceResponse<B>, actix_web::Error>> + 'static
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
{
    let request_id = RequestId::generate();
    let span = info_span!(
        "request",
        path = %request.request().path(),
        method = %request.request().method(),
        request_id = %request_id,
    );

    trace!(parent: &span, "request received");

    request.extensions_mut().insert(request_id);

    service
        .call(request)
        .instrument(span.clone())
        .inspect(|_| trace!(parent: span, "request processed"))
}
