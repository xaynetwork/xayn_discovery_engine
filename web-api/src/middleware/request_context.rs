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

use std::{future::Future, sync::Arc};

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
use reqwest::header::HeaderValue;
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
pub struct TenantId(Arc<str>);

#[derive(Debug, Error)]
#[error("TenantId is not valid: {hint:?}")]
pub struct InvalidTenantId {
    hint: String,
}

impl TenantId {
    pub fn missing() -> Self {
        static MISSING: Lazy<Arc<str>> = Lazy::new(|| "missing".into());
        TenantId(MISSING.clone())
    }

    pub fn random_legacy_tenant_id() -> Self {
        let random_id: u64 = rand::random();
        TenantId(format!("legacy.{random_id}").into_boxed_str().into())
    }

    pub fn try_parse_ascii(ascii: &[u8]) -> Result<Self, InvalidTenantId> {
        static RE: Lazy<bytes::Regex> = Lazy::new(|| {
            // printable us-ascii excluding `"`
            bytes::Regex::new(r#"^[[:print:]&&[^"]]{1,63}$"#).unwrap()
        });

        ascii.trim_ascii();

        match Uuid::try_parse_ascii(ascii) {
            Ok(id) => Ok(Self(id)),
            Err(_) => Err(InvalidTenantId {
                hint: String::from_utf8_lossy(ascii).into_owned(),
            }),
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
        .map(TenantId::try_from)
        .transpose()?;

    match header_value {
        //FIXME in follow up PR this ID will be fetched from the database
        //      during startup/storage initialization.
        None if config.enable_legacy_tenant => Ok(TenantId::missing()),
        None => Err(anyhow!("{TENANT_ID_HEADER} header missing")),
        Some(passed_value) => Ok(passed_value),
    }
}
