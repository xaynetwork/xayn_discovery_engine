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

use std::{str, sync::Arc};

use once_cell::sync::Lazy;
use regex::bytes;
use serde::{Deserialize, Serialize};
use sqlx::Type;
use thiserror::Error;

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
        Self(MISSING.clone())
    }

    pub fn random_legacy_tenant_id() -> Self {
        let random_id: u64 = rand::random();
        Self(format!("legacy.{random_id:0>16x}").as_str().into())
    }

    pub fn try_parse_ascii(ascii: &[u8]) -> Result<Self, InvalidTenantId> {
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
