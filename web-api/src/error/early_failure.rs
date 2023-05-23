// Copyright 2023 Xayn AG
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

use std::fmt::Display;

use actix_web::{
    body::BoxBody,
    dev::{ServiceRequest, ServiceResponse},
};
use tracing::Level;
use xayn_web_api_shared::request::TenantId;

use crate::{
    error::{application::application_event, json_error::JsonErrorResponseBuilder},
    middleware::request_context::RequestId,
};

pub(crate) fn middleware_failure(
    middleware: &'static str,
    request: ServiceRequest,
    request_id: Option<RequestId>,
    tenant_id: Option<TenantId>,
    event: impl Display,
    level: Level,
) -> ServiceResponse<BoxBody> {
    let request_id = request_id.unwrap_or_else(RequestId::missing);
    let tenant_id = tenant_id.unwrap_or_else(TenantId::missing);

    application_event!(
        level,
        middleware,
        path = %request.request().path(),
        method = %request.request().method(),
        %request_id,
        %tenant_id,
        %event,
    );

    let (request, _) = request.into_parts();
    ServiceResponse::new(
        request,
        JsonErrorResponseBuilder::internal_server_error(request_id),
    )
}
