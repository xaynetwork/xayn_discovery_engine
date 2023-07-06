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

use secrecy::Secret;
use serde::{Serialize, Serializer};
use serde_json::Value;

/// Serialize a `Secret<String>` as `"[REDACTED]"`.
pub fn serialize_redacted<S>(_secret: &Secret<String>, serializer: S) -> Result<S::Ok, S::Error>
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

pub mod serde_duration_as_seconds {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(deserializer).map(Duration::from_secs)
    }
}

#[macro_export]
macro_rules! json_object {
    ({ $($tt:tt)* }) => ({
        let ::serde_json::Value::Object(object) = json!({ $($tt)* }) else {
            ::std::unreachable!(/* the {} enforces it's always an object */);
        };
        object
    });
}

pub use json_object;

#[macro_export]
macro_rules! json_array {
    ([$($tt:tt)*]) => ({
        let ::serde_json::Value::Array(array) = json!([$($tt)*]) else {
            ::std::unreachable!(/* the [] enforces it's always an array */);
        };
        array
    });
}

pub use json_array;

pub type JsonObject = serde_json::Map<String, Value>;

pub fn merge_json_objects(objects: impl IntoIterator<Item = JsonObject>) -> JsonObject {
    objects
        .into_iter()
        .reduce(|mut acc, obj| {
            acc.extend(obj);
            acc
        })
        .unwrap_or_default()
}
