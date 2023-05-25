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

use std::{
    str::{self, FromStr},
    sync::Arc,
};

use derive_more::{AsRef, From};
use once_cell::sync::Lazy;
use regex::bytes;
use serde::{Deserialize, Serialize};
use sqlx::{
    postgres::{PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueRef},
    Decode,
    Encode,
    Postgres,
    Type,
};
use thiserror::Error;

#[derive(
    Clone, Debug, derive_more::Display, From, AsRef, PartialEq, Eq, Hash, Deserialize, Serialize,
)]
#[as_ref(forward)]
#[serde(transparent)]
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

impl FromStr for TenantId {
    type Err = InvalidTenantId;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        TenantId::try_parse_ascii(s.as_bytes())
    }
}

// ---- below are by hand implementations of derive(sqlx::Type) -------
// this is needed as it doesn't work with Arc<str>

impl<'q> Encode<'q, Postgres> for TenantId {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> ::sqlx::encode::IsNull {
        <&str as Encode<'q, Postgres>>::encode_by_ref(&&*self.0, buf)
    }

    fn produces(&self) -> Option<PgTypeInfo> {
        <&str as Encode<'q, Postgres>>::produces(&&*self.0)
    }
    fn size_hint(&self) -> usize {
        <&str as Encode<'q, Postgres>>::size_hint(&&*self.0)
    }
}

impl<'r> Decode<'r, Postgres> for TenantId {
    fn decode(
        value: PgValueRef<'r>,
    ) -> Result<Self, Box<dyn ::std::error::Error + 'static + Send + Sync>> {
        Ok(Self::try_parse_ascii(value.as_bytes()?)?)
    }
}

impl Type<Postgres> for TenantId {
    fn type_info() -> PgTypeInfo {
        <&str as Type<Postgres>>::type_info()
    }
    fn compatible(ty: &PgTypeInfo) -> bool {
        <&str as Type<Postgres>>::compatible(ty)
    }
}

impl PgHasArrayType for TenantId {
    fn array_type_info() -> PgTypeInfo {
        <&str as PgHasArrayType>::array_type_info()
    }
}
