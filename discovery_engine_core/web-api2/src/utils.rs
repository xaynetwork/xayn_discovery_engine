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

use secrecy::Secret;
use serde::Serializer;

/// Serialize a `Secret<String>` as `"[REDACTED]"`.
pub(crate) fn serialize_redacted<S>(
    _secret: &Secret<String>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str("[REDACTED]")
}

/// Serialize a `Option<Secret<String>>` as `Some("[REDACTED]")` or `None`.
pub(crate) fn serialize_redacted_opt<S>(
    secret: &Option<Secret<String>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if secret.is_some() {
        serializer.serialize_some("[REDACTED]")
    } else {
        serializer.serialize_none()
    }
}
