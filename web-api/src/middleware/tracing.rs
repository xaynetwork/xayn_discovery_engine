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

/// Like [`HttpServer.new()`] but sets things up to use a given non-global tracing subscriber.
///
/// The wrapper will do following things:
///
/// 1. wrap the call of the factory function passed to [`HttpServer.new()`] so that
///    the right subscriber is active during that call.
/// 2. add a middleware which
///     a. provides the right subscriber to all middleware calls (e.g. a `warp_fn` function
///         will have the right logging context even before calling `service.call(request)`)
///     b. wraps the build request future so that it has the right logging context when resolving
// Hint: Making this a macro saves us very complex nightmare of type annotations (and also way less lines of code).
macro_rules! new_http_server_with_subscriber {
    ($subscriber:expr, move || $factory:block) => {{
        use ::actix_web::dev::Service;
        use ::tracing::{dispatcher, instrument::WithSubscriber, Dispatch};
        let subscriber = Dispatch::from($subscriber);
        HttpServer::new(move || {
            // Hint: Makes sure the factory has the right dispatcher.
            dispatcher::with_default(&subscriber, || {
                let subscriber = subscriber.clone();
                $factory.wrap_fn(move |r, s| {
                    // Hint: Makes sure the middleware code wrapping the request
                    //       have the right dispatcher.
                    let fut = dispatcher::with_default(&subscriber, || s.call(r));
                    // Hint: Makes sure the wrapped request has the right dispatcher.
                    fut.with_subscriber(subscriber.clone())
                })
            })
        })
    }};
}

pub(crate) use new_http_server_with_subscriber;
