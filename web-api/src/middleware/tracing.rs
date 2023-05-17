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

use actix_web::{
    body::MessageBody,
    dev::{Service, ServiceRequest, ServiceResponse},
};
use futures_util::{future::LocalBoxFuture, FutureExt};
use tracing::{dispatcher, instrument::WithSubscriber, Dispatch};

/// Creates a wrapper which can be passed to `wrap_fn` which setup up local trace event dispatch.
///
/// When building a actix http server app apply this last, i.e. make it the
/// "outer most" wrapper.
///
/// The wrapper will do two things:
///
/// 1. Sync wrap `service.call(request)` call to make sure future middleware
///    has the right logging context when being called (e.g. more inner `wrap_fn`
///    function calls).
///
/// 2. Async wrap the result of `service.call(request)` to make sure all  requests
///     and wrapped post-request middleware handling hath the right logging context.
pub(crate) fn create_wrapper_for_local_trace_event_dispatch<S, B>(
    subscriber: impl Into<Dispatch>,
) -> impl Fn(
    ServiceRequest,
    &S,
) -> LocalBoxFuture<'static, Result<ServiceResponse<B>, actix_web::Error>>
       + Clone
       + 'static
where
    B: MessageBody,
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
{
    let subscriber = subscriber.into();
    move |r, s| {
        let fut = dispatcher::with_default(&subscriber, || s.call(r));
        fut.with_subscriber(subscriber.clone()).boxed_local()
    }
}
