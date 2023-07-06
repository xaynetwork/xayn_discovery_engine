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

use std::fmt;

use serde::{
    de::{Error, MapAccess, Unexpected, Visitor},
    Deserialize,
    Deserializer,
};

use crate::models::{DocumentProperty, DocumentPropertyId};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub(crate) enum CompareOp {
    #[serde(rename = "$eq")]
    Eq,
    #[serde(rename = "$in")]
    In,
}

#[derive(Clone, Debug, PartialEq)]
struct CompareWith {
    operation: CompareOp,
    value: DocumentProperty,
}

impl<'de> Deserialize<'de> for CompareWith {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CompareWithVisitor;

        impl<'de> Visitor<'de> for CompareWithVisitor {
            type Value = CompareWith;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a json object with exactly one right comparison argument and a matching type for the comparison operator")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                if let Some(size) = map.size_hint() {
                    if size != 1 {
                        return Err(A::Error::invalid_length(size, &Self));
                    }
                }
                let Some((operation, value)) = map.next_entry::<_, DocumentProperty>()? else {
                    return Err(A::Error::invalid_length(0, &Self));
                };
                if map.next_entry::<CompareOp, DocumentProperty>()?.is_some() {
                    return Err(A::Error::invalid_length(2, &Self));
                }
                match operation {
                    CompareOp::Eq => {
                        if !value.is_string() {
                            return Err(A::Error::invalid_type(
                                Unexpected::Other("no string property"),
                                &Self,
                            ));
                        }
                    }
                    CompareOp::In => {
                        if value
                            .as_array()
                            .map_or(true, |value| value.iter().any(|value| !value.is_string()))
                        {
                            return Err(A::Error::invalid_type(
                                Unexpected::Other("no array of strings property"),
                                &Self,
                            ));
                        }
                        let len = value.as_array().unwrap(/* value is array */).len();
                        if len > 10 {
                            return Err(A::Error::invalid_length(len, &Self));
                        }
                    }
                }

                Ok(CompareWith { operation, value })
            }
        }

        deserializer.deserialize_map(CompareWithVisitor)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Compare {
    pub(crate) operation: CompareOp,
    pub(crate) field: DocumentPropertyId,
    pub(crate) value: DocumentProperty,
}

impl<'de> Deserialize<'de> for Compare {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CompareVisitor;

        impl<'de> Visitor<'de> for CompareVisitor {
            type Value = Compare;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a json object with exactly one left comparison argument")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                if let Some(size) = map.size_hint() {
                    if size != 1 {
                        return Err(A::Error::invalid_length(size, &Self));
                    }
                }
                let Some((field, compare_with)) = map.next_entry::<_, CompareWith>()? else {
                    return Err(A::Error::invalid_length(0, &Self));
                };
                if map
                    .next_entry::<DocumentPropertyId, CompareWith>()?
                    .is_some()
                {
                    return Err(A::Error::invalid_length(2, &Self));
                }

                Ok(Compare {
                    operation: compare_with.operation,
                    field,
                    value: compare_with.value,
                })
            }
        }

        deserializer.deserialize_map(CompareVisitor)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub(crate) enum Filter {
    Compare(Compare),
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_compare_with_string() {
        assert_eq!(
            serde_json::from_str::<CompareWith>(r#"{ "$eq": "test" }"#).unwrap(),
            CompareWith {
                operation: CompareOp::Eq,
                value: json!("test").try_into().unwrap(),
            },
        );
        assert_eq!(
            serde_json::from_str::<CompareWith>("{}")
                .unwrap_err()
                .to_string(),
            "invalid length 0, expected a json object with exactly one right comparison argument and a matching type for the comparison operator at line 1 column 2",
        );
        assert_eq!(
            serde_json::from_str::<CompareWith>(r#"{ "$eq": "test1", "$eq": "test2" }"#)
                .unwrap_err()
                .to_string(),
            "invalid length 2, expected a json object with exactly one right comparison argument and a matching type for the comparison operator at line 1 column 34",
        );
        assert_eq!(
            serde_json::from_str::<CompareWith>(r#"{ "$eq": 42 }"#)
                .unwrap_err()
                .to_string(),
            "invalid type: no string property, expected a json object with exactly one right comparison argument and a matching type for the comparison operator at line 1 column 13",
        );
    }

    #[test]
    fn test_compare_with_array_string() {
        assert_eq!(
            serde_json::from_str::<CompareWith>(r#"{ "$in": ["test", "this"] }"#).unwrap(),
            CompareWith {
                operation: CompareOp::In,
                value: json!(["test", "this"]).try_into().unwrap(),
            },
        );
        assert_eq!(
            serde_json::from_str::<CompareWith>("{}")
                .unwrap_err()
                .to_string(),
            "invalid length 0, expected a json object with exactly one right comparison argument and a matching type for the comparison operator at line 1 column 2",
        );
        assert_eq!(
            serde_json::from_str::<CompareWith>(
                r#"{ "$in": ["test", "this"], "$in": ["test", "that"] }"#
            )
            .unwrap_err()
            .to_string(),
            "invalid length 2, expected a json object with exactly one right comparison argument and a matching type for the comparison operator at line 1 column 52",
        );
        assert_eq!(
            serde_json::from_str::<CompareWith>(r#"{ "$in": "test" }"#)
                .unwrap_err()
                .to_string(),
            "invalid type: no array of strings property, expected a json object with exactly one right comparison argument and a matching type for the comparison operator at line 1 column 17",
        );
        assert_eq!(
            serde_json::from_str::<CompareWith>(
                r#"{ "$in": ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11"] }"#
            )
            .unwrap_err()
            .to_string(),
            "invalid length 11, expected a json object with exactly one right comparison argument and a matching type for the comparison operator at line 1 column 68",
        );
    }

    #[test]
    fn test_compare() {
        assert_eq!(
            serde_json::from_str::<Compare>(r#"{ "prop": { "$eq": "test" } }"#).unwrap(),
            Compare {
                operation: CompareOp::Eq,
                field: "prop".try_into().unwrap(),
                value: json!("test").try_into().unwrap(),
            },
        );
        assert_eq!(
            serde_json::from_str::<Compare>(r#"{ "prop": { "$in": ["test", "this"] } }"#).unwrap(),
            Compare {
                operation: CompareOp::In,
                field: "prop".try_into().unwrap(),
                value: json!(["test", "this"]).try_into().unwrap(),
            },
        );
        assert_eq!(
            serde_json::from_str::<Compare>("{}")
                .unwrap_err()
                .to_string(),
            "invalid length 0, expected a json object with exactly one left comparison argument at line 1 column 2",
        );
        assert_eq!(
            serde_json::from_str::<Compare>(
                r#"{ "prop1": { "$eq": "test" }, "prop2": { "$in": ["test", "this"] } }"#
            )
            .unwrap_err()
            .to_string(),
            "invalid length 2, expected a json object with exactly one left comparison argument at line 1 column 68",
        );
    }
}
