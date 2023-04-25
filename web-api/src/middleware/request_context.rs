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

use std::{future::Future, str, sync::Arc};

use actix_web::{
    body::BoxBody,
    dev::{Service, ServiceRequest, ServiceResponse},
    web::Data,
    HttpMessage,
    HttpRequest,
};
use anyhow::anyhow;
use futures_util::{
    future::{self, Either},
    FutureExt,
};
use once_cell::sync::Lazy;
use regex::bytes;
use serde::{Deserialize, Serialize};
use sqlx::Type;
use thiserror::Error;
use tracing::{error_span, instrument, trace, Instrument};
use uuid::Uuid;

use crate::{error::early_failure::middleware_failure, tenants};

pub(crate) struct RequestContext {
    #[allow(unused)]
    pub(crate) tenant_id: TenantId,
    pub(crate) request_id: RequestId,
}

impl RequestContext {
    /// Tries to return the current [`RequestContext`] based on a request.
    ///
    /// The context will be setup by the `setup_request_context` middleware.
    #[instrument(skip_all, err)]
    pub(crate) fn try_extract_from_request<R>(
        request: &HttpRequest,
        func: impl FnOnce(&Arc<RequestContext>) -> R,
    ) -> Result<R, AccessError> {
        let extensions = request.extensions();
        Ok(func(extensions.get::<Arc<RequestContext>>().ok_or(
            AccessError {
                method: "try_extract_from_request",
            },
        )?))
    }
}

#[derive(Debug, Error)]
#[error("Failed to access expected context value in: {method}")]
pub(crate) struct AccessError {
    method: &'static str,
}

#[derive(
    Clone,
    Debug,
    derive_more::Display,
    derive_more::From,
    PartialEq,
    Eq,
    Hash,
    Deserialize,
    Serialize,
    Type,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub(crate) struct TenantId(Arc<str>);

#[derive(Debug, Error)]
#[error("TenantId is not valid: {hint:?}")]
pub(crate) struct InvalidTenantId {
    hint: String,
}

impl TenantId {
    pub(crate) fn missing() -> Self {
        static MISSING: Lazy<Arc<str>> = Lazy::new(|| "missing".into());
        Self(MISSING.clone())
    }

    #[allow(dead_code)]
    fn random_legacy_tenant_id() -> Self {
        let random_id: u64 = rand::random();
        Self(format!("legacy.{random_id:0>16x}").as_str().into())
    }

    fn try_parse_ascii(ascii: &[u8]) -> Result<Self, InvalidTenantId> {
        static RE: Lazy<bytes::Regex> =
            Lazy::new(|| bytes::Regex::new(r"^[a-zA-Z0-9_:@.-]{1,50}$").unwrap());

        if RE.is_match(ascii) {
            Ok(Self(
                str::from_utf8(ascii).unwrap(/*regex guarantees valid utf-8*/).into(),
            ))
        } else {
            Err(InvalidTenantId {
                hint: String::from_utf8_lossy(ascii).into_owned(),
            })
        }
    }
}

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

/// Sets up the call context.
///
/// This makes the `RequestId` and `TenantId` available as extensions and sets up tracing for all calls.
///
/// The `TenantId` is required.
pub(crate) fn setup_request_context<A, S>(
    request: ServiceRequest,
    service: &S,
) -> impl Future<Output = Result<ServiceResponse<BoxBody>, actix_web::Error>> + 'static
where
    A: AsRef<tenants::Config> + 'static,
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = actix_web::Error>,
    S::Future: 'static,
{
    let request_id = RequestId::generate();
    let config = request.app_data::<Data<A>>().unwrap().get_ref().as_ref();

    let tenant_id = match extract_tenant_id(config, &request) {
        Ok(id) => id,
        Err(error) => {
            let response = middleware_failure(
                "setup_request_context",
                request,
                Some(request_id),
                None,
                error,
            );
            return Either::Left(future::ok(response));
        }
    };

    // the request span must have the lowest level, otherwise it will not be added to the logs if a
    // subscriber with a lower level filter than the span level is used
    let span = error_span!(
        "request",
        path = %request.request().path(),
        method = %request.request().method(),
        %request_id,
        %tenant_id,
    );

    trace!(parent: &span, "request received");

    let context = Arc::new(RequestContext {
        tenant_id,
        request_id,
    });

    request.extensions_mut().insert(context);

    Either::Right(
        service
            .call(request)
            .instrument(span.clone())
            .inspect(|_| trace!(parent: span, "request processed")),
    )
}

const TENANT_ID_HEADER: &str = "X-Tenant-Id";

fn extract_tenant_id(
    config: &tenants::Config,
    request: &ServiceRequest,
) -> Result<TenantId, anyhow::Error> {
    let header_value = request
        .headers()
        .get(TENANT_ID_HEADER)
        .map(|value| TenantId::try_parse_ascii(trim_ascii(value.as_bytes())))
        .transpose()?;

    match header_value {
        //FIXME in follow up PR this ID will be fetched from the database
        //      during startup/storage initialization.
        None if config.enable_legacy_tenant => Ok(TenantId::missing()),
        None => Err(anyhow!("{TENANT_ID_HEADER} header missing")),
        Some(passed_value) => Ok(passed_value),
    }
}

//FIXME use <&[u8]>::trim_ascii() once stabilized
//  https://github.com/rust-lang/rust/issues/94035
fn trim_ascii(ascii: &[u8]) -> &[u8] {
    trim_ascii_end(trim_ascii_start(ascii))
}

fn trim_ascii_start(ascii: &[u8]) -> &[u8] {
    ascii
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .map_or(&[], |new_first| &ascii[new_first..])
}

fn trim_ascii_end(ascii: &[u8]) -> &[u8] {
    ascii
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(&[], |new_last| &ascii[..=new_last])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_ascii() {
        assert_eq!(trim_ascii(b""), b"");
        assert_eq!(trim_ascii(b" "), b"");
        assert_eq!(trim_ascii(b"ab  cd"), b"ab  cd");
        assert_eq!(trim_ascii(b"  ab  cd  "), b"ab  cd");
        assert_eq!(trim_ascii(b" \n ab\t cd  \t"), b"ab\t cd");
    }

    #[test]
    fn test_trim_ascii_start() {
        assert_eq!(trim_ascii_start(b""), b"");
        assert_eq!(trim_ascii_start(b" "), b"");
        assert_eq!(trim_ascii_start(b"ab  cd"), b"ab  cd");
        assert_eq!(trim_ascii_start(b"  ab  cd  "), b"ab  cd  ");
        assert_eq!(trim_ascii_start(b" \n ab\t cd  \t"), b"ab\t cd  \t");
    }

    #[test]
    fn test_trim_ascii_end() {
        assert_eq!(trim_ascii_end(b""), b"");
        assert_eq!(trim_ascii_end(b" "), b"");
        assert_eq!(trim_ascii_end(b"ab  cd"), b"ab  cd");
        assert_eq!(trim_ascii_end(b"  ab  cd  "), b"  ab  cd");
        assert_eq!(trim_ascii_end(b" \n ab\t cd  \t"), b" \n ab\t cd");
    }

    #[test]
    fn test_parsing_tenant_id_from_ascii() {
        assert!(TenantId::try_parse_ascii(b"").is_err());
        assert!(TenantId::try_parse_ascii(&[65u8; 50]).is_ok());
        assert!(TenantId::try_parse_ascii(&[65u8; 51]).is_err());

        TenantId::try_parse_ascii(b".:@_-").unwrap();
        TenantId::try_parse_ascii(b"aA0.9bcd").unwrap();
        TenantId::try_parse_ascii(b"abcdefghijklmnopqrstuvwxyz").unwrap();
        TenantId::try_parse_ascii(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ").unwrap();
    }
}
