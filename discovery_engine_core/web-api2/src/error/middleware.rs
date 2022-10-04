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

use std::future::Future;

use actix_web::dev::{Service, ServiceRequest, ServiceResponse};

use tracing::{debug, Instrument};
use tracing_actix_web::root_span_macro::private::tracing::info_span;
use uuid::Uuid;

use crate::error::json_wrapping::WrappedError;

pub(crate) fn json_error_bodies_middleware<S, B>(
    request: ServiceRequest,
    service: &S,
) -> impl Future<Output = Result<ServiceResponse<B>, actix_web::Error>> + 'static
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
{
    //FIXME try to use tracing-actix-web and the request id of it instead
    let request_id = Uuid::new_v4();
    let span = info_span!(
        "request",
        path = %request.request().path(),
        method = %request.request().method(),
        request_id = %request_id,
    );

    debug!(parent: &span, "request received");

    let fut = service.call(request).instrument(span.clone());
    async move {
        let res = fut.await.map_err(|mut error| {
            if crate::Error::try_inject_request_id(&mut error, request_id) {
                error
            } else {
                WrappedError { error, request_id }.into()
            }
        });

        dbg!(res.is_err());

        debug!(parent: &span, "request processed");

        res
    }
}
