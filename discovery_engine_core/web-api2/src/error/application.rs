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

use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use actix_web::{
    body::BoxBody,
    http::StatusCode,
    web::ReqData,
    FromRequest,
    Handler,
    HttpResponse,
    ResponseError,
};
use derive_more::{Deref, Display};
use futures_util::Future;
use pin_project_lite::pin_project;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use tracing::error;

use crate::middleware::tracing::RequestId;

use super::json_error::JsonErrorResponseBuilder;

#[derive(Display, Debug, Deref)]
#[display(fmt = "{}", error)]
pub struct Error {
    #[deref(forward)]
    error: Box<dyn ApplicationError>,
    request_id: RequestId,
}

impl<T> From<T> for Error
where
    T: ApplicationError,
{
    fn from(error: T) -> Self {
        Self {
            error: Box::new(error),
            request_id: RequestId::missing(),
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
            self.request_id,
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
macro_rules! impl_application_error {
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

impl_application_error!(Unimplemented => INTERNAL_SERVER_ERROR);

/// Allows to augment errors with a request id by wrapping the endpoint handler.
///
/// # Explanation/Why
///
/// This is a bit complex as it can only work when wrapping the endpoint handler
/// functions, it can't work with a middle ware nor can it work by wrapping a
/// `Service` the reason for this is that when endpoint handler functions are
/// called the result will **immediately convert `Err(error)` to an `Ok(error_response)`**
/// and we need to inject the request id before that conversion happens.
///
/// ## Comparison to other frameworks:
/// - warp, same problem but way worse as a lot of pub-traits are not pub-exposed
/// - axum, handles this nicely by a combination of allowing wrapping (layering) of
///   middleware around `Handlers` which is basically what this helper does but more
///   generic.
///
/// ## Drawbacks (compared ot axum)
///
/// - around 100 lines of future/handing wrapping and type hint magic
/// - only works with `RequestId`, axum can extract any data (we could
///   probably make this work, but it's not worth the effort)
/// - can't use `#[get(..)]` and similar
///
/// ## Alternatives
///
/// - wrap the handles with a (proc)macro (benefit compatible with `#[get(...)]`)
/// - use axum (but it's eco system is less mature and it will change over time)
/// - only include the request id in a custom header (trivial to do)
/// - land a PR in actix which add an additional `ResponseError::error_response_with_context`
///   method with an auto-forwarding default implementation which can access middleware data
pub(crate) trait WithRequestIdExt<Args>: Sized {
    fn error_with_request_id(self) -> HandlerWithIdInjection<Self, Args>;
}

impl<H, Args> WithRequestIdExt<Args> for H
where
    H: Handler<Args>,
    Args: WithRequestIdHint,
{
    fn error_with_request_id(self) -> HandlerWithIdInjection<Self, Args> {
        HandlerWithIdInjection {
            inner: self,
            args: PhantomData,
        }
    }
}

pub(crate) struct HandlerWithIdInjection<H, Args> {
    inner: H,
    args: PhantomData<Args>,
}

impl<H, Args> Clone for HandlerWithIdInjection<H, Args>
where
    H: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            args: PhantomData,
        }
    }
}

impl<H, Args, B> Handler<<Args as WithRequestIdHint>::ExtendedArgs>
    for HandlerWithIdInjection<H, Args>
where
    H: Handler<Args, Output = Result<B, Error>>,
    Args: WithRequestIdHint,
{
    type Output = H::Output;

    type Future = HandlerWithIdInjectionFut<H::Future>;

    fn call(&self, args: <Args as WithRequestIdHint>::ExtendedArgs) -> Self::Future {
        let (args, request_id) = WithRequestIdHint::strip_request_id(args);
        HandlerWithIdInjectionFut {
            inner: self.inner.call(args),
            request_id,
        }
    }
}

#[pin_project]
pub(crate) struct HandlerWithIdInjectionFut<F> {
    #[pin]
    inner: F,
    request_id: RequestId,
}

impl<F, B> Future for HandlerWithIdInjectionFut<F>
where
    F: Future<Output = Result<B, Error>>,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.inner.poll(cx).map(|value| {
            value.map_err(|mut err| {
                err.request_id = *this.request_id;
                err
            })
        })
    }
}

pub(crate) trait WithRequestIdHint: FromRequest + 'static {
    type ExtendedArgs;

    fn strip_request_id(args: Self::ExtendedArgs) -> (Self, RequestId);
}

macro_rules! impl_with_request_id_hint {
    (($first:ident)) => (
        impl_with_request_id_hint!(@impl ());
        impl_with_request_id_hint!(@impl ($first));
    );
    (($first:ident, $($name:ident),*)) => (
        impl_with_request_id_hint!(@impl ($first, $($name),*));
        impl_with_request_id_hint!(($($name),*));
    );
    (@impl ($($name:ident),*)) => (
        impl<$($name),*> WithRequestIdHint for ($($name,)*)
        where $(
            $name: FromRequest + 'static,
        )* {
            type ExtendedArgs = ($($name,)* ReqData<RequestId>,);

            fn strip_request_id(args: Self::ExtendedArgs) -> (Self, RequestId) {
                #![allow(non_snake_case)]
                let ($($name,)* last,) = args;
                (($($name,)*), *last)
            }
        }
    );
}

impl_with_request_id_hint! {
    (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
}
