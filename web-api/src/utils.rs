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

use derive_more::Deref;
use figment::value::magic::RelativePathBuf as FigmentRelativePathBuf;
use secrecy::Secret;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Serialize, Deserialize, Debug, Deref)]
#[serde(transparent)]
pub(crate) struct RelativePathBuf {
    #[serde(serialize_with = "FigmentRelativePathBuf::serialize_relative")]
    inner: FigmentRelativePathBuf,
}

impl From<&str> for RelativePathBuf {
    fn from(s: &str) -> Self {
        Self {
            inner: FigmentRelativePathBuf::from(s),
        }
    }
}

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

/// Serialize a sequence of serializable items into ndjson.
pub(crate) fn serialize_to_ndjson(
    items: impl IntoIterator<Item = Result<impl Serialize, serde_json::Error>>,
) -> Result<Vec<u8>, serde_json::Error> {
    let mut body = Vec::new();
    for item in items {
        serde_json::to_writer(&mut body, &item?)?;
        body.push(b'\n');
    }
    Ok(body)
}

pub(crate) mod serde_duration_as_seconds {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub(crate) fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(deserializer).map(Duration::from_secs)
    }
}
