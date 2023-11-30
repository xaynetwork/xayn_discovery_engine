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

use chrono::{DateTime, Utc};
use const_format::formatcp;
use derive_more::{Deref, DerefMut};
use itertools::Itertools;
use serde::{
    de::{Error, MapAccess, SeqAccess, Unexpected, Visitor},
    Deserialize,
    Deserializer,
    Serialize,
};
use serde_json::{json, Number, Value};

use crate::{
    error::common::InvalidDocumentProperty,
    models::{DocumentId, DocumentProperty, DocumentPropertyId},
    storage::property_filter::IndexedPropertiesSchema,
};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) enum CompareOp {
    #[serde(rename = "$eq")]
    Eq,
    #[serde(rename = "$in")]
    In,
    #[serde(rename = "$gt")]
    Gt,
    #[serde(rename = "$gte")]
    Gte,
    #[serde(rename = "$lt")]
    Lt,
    #[serde(rename = "$lte")]
    Lte,
}

impl CompareOp {
    const MAX_VALUES_PER_IN: usize = 500;
}

#[derive(Clone, Debug, PartialEq)]
struct CompareWith {
    operation: CompareOp,
    value: DocumentProperty,
}

impl CompareWith {
    const EXPECTING: &'static str = "a json object with exactly one right comparison argument and a matching type for the comparison operator";
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
                formatter.write_str(Self::Value::EXPECTING)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                fn unexpected_number(value: &Number) -> Unexpected<'_> {
                    if let Some(value) = value.as_u64() {
                        Unexpected::Unsigned(value)
                    } else if let Some(value) = value.as_i64() {
                        Unexpected::Signed(value)
                    } else if let Some(value) = value.as_f64() {
                        Unexpected::Float(value)
                    } else {
                        Unexpected::Other("number")
                    }
                }

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
                        if let Some(unexpected) = match &*value {
                            Value::Bool(_) | Value::String(_) => None,
                            Value::Null => Some(Unexpected::Other("null")),
                            Value::Number(value) => Some(unexpected_number(value)),
                            Value::Array(_) => Some(Unexpected::Seq),
                            Value::Object(_) => Some(Unexpected::Map),
                        } {
                            return Err(A::Error::invalid_type(unexpected, &Self));
                        }
                    }

                    CompareOp::In => {
                        if let Some(unexpected) = match &*value {
                            // we only accept string arrays as valid properties
                            Value::Array(values) => {
                                let len = values.len();
                                if len > CompareOp::MAX_VALUES_PER_IN {
                                    return Err(A::Error::invalid_length(len, &Self));
                                }
                                for value in values {
                                    if !matches!(value, Value::String(_)) {
                                        return Err(A::Error::invalid_type(
                                            Unexpected::Other("only string arrays are allowed"),
                                            &Self,
                                        ));
                                    }
                                }
                                None
                            }
                            Value::Null => Some(Unexpected::Other("null")),
                            Value::Bool(value) => Some(Unexpected::Bool(*value)),
                            Value::Number(value) => Some(unexpected_number(value)),
                            Value::String(value) => Some(Unexpected::Str(value)),
                            Value::Object(_) => Some(Unexpected::Map),
                        } {
                            return Err(A::Error::invalid_type(unexpected, &Self));
                        }
                    }

                    CompareOp::Gt | CompareOp::Gte | CompareOp::Lt | CompareOp::Lte => {
                        if let Some(unexpected) = match &*value {
                            Value::Number(_) => None,
                            Value::String(value) => DateTime::parse_from_rfc3339(value)
                                .is_err()
                                .then_some(Unexpected::Other("invalid date string")),
                            Value::Null => Some(Unexpected::Other("null")),
                            Value::Bool(value) => Some(Unexpected::Bool(*value)),
                            Value::Array(_) => Some(Unexpected::Seq),
                            Value::Object(_) => Some(Unexpected::Map),
                        } {
                            return Err(A::Error::invalid_type(unexpected, &Self));
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

impl Compare {
    const EXPECTING: &'static str = "a json object with exactly one left comparison argument";
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
                formatter.write_str(Self::Value::EXPECTING)
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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub(crate) enum CombineOp {
    #[serde(rename = "$and")]
    And,
    #[serde(rename = "$or")]
    Or,
}

#[derive(Clone, Debug, Deref, DerefMut, PartialEq)]
pub(crate) struct Filters(Vec<Filter>);

impl Filters {
    const MAX_FILTERS_PER_COMBINATION: usize = 10;
    const EXPECTING: &'static str = formatcp!(
        "a json array with at most {} combination arguments",
        Filters::MAX_FILTERS_PER_COMBINATION,
    );

    fn is_below_depth(&self, max: usize) -> bool {
        (max > 0)
            .then(|| self.iter().all(|filter| filter.is_below_depth(max - 1)))
            .unwrap_or_default()
    }
}

impl<'de> Deserialize<'de> for Filters {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FiltersVisitor;

        impl<'de> Visitor<'de> for FiltersVisitor {
            type Value = Filters;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(Self::Value::EXPECTING)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let size = seq.size_hint().unwrap_or_default();
                if size > Self::Value::MAX_FILTERS_PER_COMBINATION {
                    return Err(A::Error::invalid_length(size, &Self));
                }

                let mut filters = Vec::with_capacity(size);
                while let Some(filter) = seq.next_element()? {
                    if filters.len() < Self::Value::MAX_FILTERS_PER_COMBINATION {
                        filters.push(filter);
                    } else {
                        return Err(A::Error::invalid_length(filters.len() + 1, &Self));
                    }
                }

                Ok(Filters(filters))
            }
        }

        deserializer.deserialize_seq(FiltersVisitor)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Combine {
    pub(crate) operation: CombineOp,
    pub(crate) filters: Filters,
}

impl Combine {
    const MAX_DEPTH: usize = 2;
    const EXPECTING: &'static str = formatcp!(
        "a json object with exactly one combination operator and at most {} times nested combinations",
        Combine::MAX_DEPTH,
    );
    const UNEXPECTED_DEPTH: &'static str =
        formatcp!("more than {} times nested combinations", Combine::MAX_DEPTH);
}

impl<'de> Deserialize<'de> for Combine {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CombineVisitor;

        impl<'de> Visitor<'de> for CombineVisitor {
            type Value = Combine;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(Self::Value::EXPECTING)
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
                let Some((operation, filters)) = map.next_entry::<_, Filters>()? else {
                    return Err(A::Error::invalid_length(0, &Self));
                };
                if map.next_entry::<CombineOp, Filters>()?.is_some() {
                    return Err(A::Error::invalid_length(2, &Self));
                }
                if !filters.is_below_depth(Self::Value::MAX_DEPTH) {
                    return Err(A::Error::invalid_value(
                        Unexpected::Other(Self::Value::UNEXPECTED_DEPTH),
                        &Self,
                    ));
                }

                Ok(Combine { operation, filters })
            }
        }

        deserializer.deserialize_map(CombineVisitor)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Ids {
    pub(crate) ids: Vec<DocumentId>,
}

impl<'de> Deserialize<'de> for Ids {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SerdeProxy {
            #[serde(rename = "$ids")]
            pub(crate) ids: Vec<String>,
        }

        let SerdeProxy { ids } = SerdeProxy::deserialize(deserializer)?;

        let len = ids.len();
        let max = CompareOp::MAX_VALUES_PER_IN;
        if len > max {
            return Err(D::Error::invalid_length(
                len,
                &format!("$ids must contain 0..={max} ids").as_str(),
            ));
        }

        // HINT: (de-)serialization doesn't check id validity as it can't differentiate between from-db
        //       deserialization and query filter deserialization so we need to run a post validation step
        let ids = ids
            .into_iter()
            .map(|id| {
                DocumentId::new(&id).map_err(|_| {
                    D::Error::invalid_value(Unexpected::Str(&id), &"a valid DocumentId")
                })
            })
            .try_collect()?;

        Ok(Self { ids })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Filter {
    Compare(Compare),
    Combine(Combine),
    Ids(Ids),
}

impl Filter {
    fn is_below_depth(&self, max: usize) -> bool {
        match self {
            Self::Compare(_) | Self::Ids(_) => true,
            Self::Combine(combine) => combine.filters.is_below_depth(max),
        }
    }

    pub(crate) fn insert_published_after(
        filter: Option<Self>,
        published_after: Option<DateTime<Utc>>,
    ) -> Option<Self> {
        if let Some(published_after) = published_after {
            let field = "publication_date".try_into().unwrap(/* valid property id */);
            let date = published_after.to_rfc3339();
            let value = DocumentProperty::try_from_value(
                &field,
                json!(date),
                date.len()
            ).unwrap(/* valid property */);
            let published_after = Self::Compare(Compare {
                operation: CompareOp::Gte,
                field,
                value,
            });

            let filter = if let Some(filter) = filter {
                match filter {
                    compare @ Self::Compare(_) => Self::Combine(Combine {
                        operation: CombineOp::And,
                        filters: Filters(vec![compare, published_after]),
                    }),
                    ids @ Self::Ids(_) => Self::Combine(Combine {
                        operation: CombineOp::And,
                        filters: Filters(vec![ids, published_after]),
                    }),
                    Self::Combine(mut combine) if matches!(combine.operation, CombineOp::And) => {
                        combine.filters.push(published_after);
                        Self::Combine(combine)
                    }
                    combine @ Self::Combine(_) => Self::Combine(Combine {
                        operation: CombineOp::And,
                        filters: Filters(vec![combine, published_after]),
                    }),
                }
            } else {
                published_after
            };

            Some(filter)
        } else {
            filter
        }
    }

    pub(crate) fn validate(
        &self,
        schema: &IndexedPropertiesSchema,
    ) -> Result<(), InvalidDocumentProperty> {
        match self {
            Self::Ids(_) => Ok(()),
            Self::Compare(compare) => schema.validate_filter(&compare.field, &compare.value),
            Self::Combine(combine) => combine
                .filters
                .iter()
                .map(|filter| filter.validate(schema))
                .try_collect(),
        }
    }
}

impl<'de> Deserialize<'de> for Filter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // this is essentially what #[serde(untagged)] generates, but with better error messages
        // than the rather inexpressive "data did not match any variant of untagged enum Filter"
        use serde::__private::de::{Content, ContentRefDeserializer};

        let filter = Content::deserialize(deserializer)?;
        let deserializer = ContentRefDeserializer::<D::Error>::new(&filter);

        let compare = match Compare::deserialize(deserializer) {
            Ok(compare) => return Ok(Filter::Compare(compare)),
            Err(error) => error,
        };
        let combine = match Combine::deserialize(deserializer) {
            Ok(combine) => return Ok(Filter::Combine(combine)),
            Err(error) => error,
        };
        let ids = match Ids::deserialize(deserializer) {
            Ok(ids) => return Ok(Filter::Ids(ids)),
            Err(error) => error,
        };

        Err(D::Error::custom(format!(
            "invalid variant, expected one of: Compare({compare}); Combine({combine}); Ids({ids})",
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fmt::Debug};

    use serde::de::DeserializeOwned;
    use serde_json::json;

    use super::*;
    use crate::{
        error::common::InvalidDocumentPropertyReason,
        storage::property_filter::{IndexedPropertyDefinition, IndexedPropertyType},
    };

    const DATE: &str = "1234-05-06T07:08:09Z";

    trait Expecting {
        const EXPECTING: &'static str;
    }

    impl Expecting for CompareWith {
        const EXPECTING: &'static str = Self::EXPECTING;
    }

    impl Expecting for Compare {
        const EXPECTING: &'static str = Self::EXPECTING;
    }

    impl Expecting for Combine {
        const EXPECTING: &'static str = Self::EXPECTING;
    }

    fn assert_invalid_len<T>(filter: &str, len: usize, column: usize)
    where
        T: Debug + DeserializeOwned + Expecting,
    {
        let error = serde_json::from_str::<T>(filter).unwrap_err().to_string();
        let message = format!(
            "invalid length {len}, expected {} at line 1 column {column}",
            T::EXPECTING,
        );
        assert_eq!(error, message);
    }

    fn assert_invalid_type(filter: &str, value: &str, column: usize) {
        let error = serde_json::from_str::<CompareWith>(filter)
            .unwrap_err()
            .to_string();
        let message = format!(
            "invalid type: {value}, expected {} at line 1 column {column}",
            CompareWith::EXPECTING,
        );
        assert_eq!(error, message);
    }

    #[test]
    fn test_compare_with_null() {
        assert_invalid_len::<CompareWith>("{}", 0, 2);
        assert_invalid_len::<CompareWith>(r#"{ "$eq": null, "$eq": null }"#, 2, 28);
        assert_invalid_type(r#"{ "$in": null }"#, "null", 15);
        assert_invalid_type(r#"{ "$gt": null }"#, "null", 15);
        assert_invalid_type(r#"{ "$gte": null }"#, "null", 16);
        assert_invalid_type(r#"{ "$lt": null }"#, "null", 15);
        assert_invalid_type(r#"{ "$lte": null }"#, "null", 16);
    }

    #[test]
    fn test_compare_with_bool() {
        assert_eq!(
            serde_json::from_str::<CompareWith>(r#"{ "$eq": true }"#).unwrap(),
            CompareWith {
                operation: CompareOp::Eq,
                value: json!(true).try_into().unwrap(),
            },
        );
        assert_eq!(
            serde_json::from_str::<CompareWith>(r#"{ "$eq": false }"#).unwrap(),
            CompareWith {
                operation: CompareOp::Eq,
                value: json!(false).try_into().unwrap(),
            },
        );
        assert_invalid_len::<CompareWith>("{}", 0, 2);
        assert_invalid_len::<CompareWith>(r#"{ "$eq": true, "$eq": false }"#, 2, 29);
        assert_invalid_type(r#"{ "$in": true }"#, "boolean `true`", 15);
        assert_invalid_type(r#"{ "$gt": true }"#, "boolean `true`", 15);
        assert_invalid_type(r#"{ "$gte": true }"#, "boolean `true`", 16);
        assert_invalid_type(r#"{ "$lt": true }"#, "boolean `true`", 15);
        assert_invalid_type(r#"{ "$lte": true }"#, "boolean `true`", 16);
    }

    #[test]
    fn test_compare_with_string() {
        assert_eq!(
            serde_json::from_str::<CompareWith>(r#"{ "$eq": "abc" }"#).unwrap(),
            CompareWith {
                operation: CompareOp::Eq,
                value: json!("abc").try_into().unwrap(),
            },
        );
        assert_invalid_len::<CompareWith>("{}", 0, 2);
        assert_invalid_len::<CompareWith>(r#"{ "$eq": "abc", "$eq": "def" }"#, 2, 30);
        assert_invalid_type(r#"{ "$in": "abc" }"#, r#"string "abc""#, 16);
        assert_invalid_type(r#"{ "$gt": "abc" }"#, "invalid date string", 16);
        assert_invalid_type(r#"{ "$gte": "abc" }"#, "invalid date string", 17);
        assert_invalid_type(r#"{ "$lt": "abc" }"#, "invalid date string", 16);
        assert_invalid_type(r#"{ "$lte": "abc" }"#, "invalid date string", 17);
    }

    #[test]
    fn test_compare_with_array_string() {
        assert_eq!(
            serde_json::from_str::<CompareWith>(r#"{ "$in": ["a", "b", "c"] }"#).unwrap(),
            CompareWith {
                operation: CompareOp::In,
                value: json!(["a", "b", "c"]).try_into().unwrap(),
            },
        );
        assert_invalid_len::<CompareWith>("{}", 0, 2);
        assert_invalid_len::<CompareWith>(
            r#"{ "$in": ["a", "b", "c"], "$in": ["d", "e", "f"] }"#,
            2,
            50,
        );
        assert_invalid_len::<CompareWith>(
            &json!({ "$in": vec!["a"; CompareOp::MAX_VALUES_PER_IN + 1] }).to_string(),
            CompareOp::MAX_VALUES_PER_IN + 1,
            2013,
        );
        assert_invalid_type(r#"{ "$in": [null] }"#, "only string arrays are allowed", 17);
        assert_invalid_type(r#"{ "$in": [0] }"#, "only string arrays are allowed", 14);
        assert_invalid_type(r#"{ "$in": [1.0] }"#, "only string arrays are allowed", 16);
        assert_invalid_type(r#"{ "$in": [true] }"#, "only string arrays are allowed", 17);
        assert_invalid_type(r#"{ "$in": [[]] }"#, "only string arrays are allowed", 15);
        //assert_invalid_type(r#"{ "$in": [{}}] }"#, "only string arrays are allowed", 15);
        assert_invalid_type(r#"{ "$eq": ["a", "b", "c"] }"#, "sequence", 26);
        assert_invalid_type(r#"{ "$gt": ["a", "b", "c"] }"#, "sequence", 26);
        assert_invalid_type(r#"{ "$gte": ["a", "b", "c"] }"#, "sequence", 27);
        assert_invalid_type(r#"{ "$lt": ["a", "b", "c"] }"#, "sequence", 26);
        assert_invalid_type(r#"{ "$lte": ["a", "b", "c"] }"#, "sequence", 27);
    }

    #[test]
    fn test_compare_with_number() {
        for (op, operation) in [
            ("$gt", CompareOp::Gt),
            ("$gte", CompareOp::Gte),
            ("$lt", CompareOp::Lt),
            ("$lte", CompareOp::Lte),
        ] {
            for number in [json!(42_u64), json!(-42_i64), json!(42_f64)] {
                assert_eq!(
                    serde_json::from_str::<CompareWith>(&json!({ op: number }).to_string())
                        .unwrap(),
                    CompareWith {
                        operation,
                        value: number.try_into().unwrap(),
                    },
                );
            }
        }
        assert_invalid_len::<CompareWith>("{}", 0, 2);
        assert_invalid_len::<CompareWith>(r#"{ "$gt": 42, "$lt": 42 }"#, 2, 24);
        assert_invalid_type(r#"{ "$eq": 42 }"#, "integer `42`", 13);
        assert_invalid_type(r#"{ "$in": 42 }"#, "integer `42`", 13);
    }

    #[test]
    fn test_compare_with_date() {
        assert_eq!(
            serde_json::from_str::<CompareWith>(&json!({ "$eq": DATE }).to_string()).unwrap(),
            CompareWith {
                operation: CompareOp::Eq,
                value: json!(DATE).try_into().unwrap(),
            },
        );
        for (op, operation) in [
            ("$gt", CompareOp::Gt),
            ("$gte", CompareOp::Gte),
            ("$lt", CompareOp::Lt),
            ("$lte", CompareOp::Lte),
        ] {
            assert_eq!(
                serde_json::from_str::<CompareWith>(&json!({ op: DATE }).to_string()).unwrap(),
                CompareWith {
                    operation,
                    value: json!(DATE).try_into().unwrap(),
                },
            );
        }
        assert_invalid_len::<CompareWith>("{}", 0, 2);
        assert_invalid_len::<CompareWith>(&json!({ "$gt": DATE, "$lt": DATE }).to_string(), 2, 59);
        assert_invalid_type(
            &json!({ "$in": DATE }).to_string(),
            &format!(r#"string "{DATE}""#),
            30,
        );
    }

    #[test]
    fn test_compare() {
        assert_eq!(
            serde_json::from_str::<Compare>(r#"{ "prop": { "$eq": true } }"#).unwrap(),
            Compare {
                operation: CompareOp::Eq,
                field: "prop".try_into().unwrap(),
                value: json!(true).try_into().unwrap(),
            },
        );
        assert_eq!(
            serde_json::from_str::<Compare>(r#"{ "prop": { "$eq": "abc" } }"#).unwrap(),
            Compare {
                operation: CompareOp::Eq,
                field: "prop".try_into().unwrap(),
                value: json!("abc").try_into().unwrap(),
            },
        );
        assert_eq!(
            serde_json::from_str::<Compare>(r#"{ "prop": { "$in": ["a", "b", "c"] } }"#).unwrap(),
            Compare {
                operation: CompareOp::In,
                field: "prop".try_into().unwrap(),
                value: json!(["a", "b", "c"]).try_into().unwrap(),
            },
        );
        assert_eq!(
            serde_json::from_str::<Compare>(r#"{ "prop": { "$gt": 42 } }"#).unwrap(),
            Compare {
                operation: CompareOp::Gt,
                field: "prop".try_into().unwrap(),
                value: json!(42).try_into().unwrap(),
            },
        );
        assert_eq!(
            serde_json::from_str::<Compare>(&json!({ "prop": { "$gt": DATE } }).to_string())
                .unwrap(),
            Compare {
                operation: CompareOp::Gt,
                field: "prop".try_into().unwrap(),
                value: json!(DATE).try_into().unwrap(),
            },
        );
        assert_invalid_len::<Compare>("{}", 0, 2);
        assert_invalid_len::<Compare>(
            r#"{ "prop1": { "$eq": "a" }, "prop2": { "$in": ["b", "c"] } }"#,
            2,
            59,
        );
    }

    #[test]
    fn test_is_below_depth() {
        let compare = Filter::Compare(Compare {
            operation: CompareOp::Eq,
            field: "prop".try_into().unwrap(),
            value: json!("test").try_into().unwrap(),
        });
        assert!(compare.is_below_depth(0));

        let filters = Filters(Vec::new());
        assert!(!filters.is_below_depth(0));
        assert!(filters.is_below_depth(1));
        let combine_0 = Filter::Combine(Combine {
            operation: CombineOp::And,
            filters,
        });
        assert!(!combine_0.is_below_depth(0));
        assert!(combine_0.is_below_depth(1));

        let filters = Filters(vec![compare.clone()]);
        assert!(!filters.is_below_depth(0));
        assert!(filters.is_below_depth(1));
        let combine_1 = Filter::Combine(Combine {
            operation: CombineOp::And,
            filters,
        });
        assert!(!combine_1.is_below_depth(0));
        assert!(combine_1.is_below_depth(1));

        let filters = Filters(vec![compare.clone(), combine_0]);
        assert!(!filters.is_below_depth(1));
        assert!(filters.is_below_depth(2));
        let combine = Filter::Combine(Combine {
            operation: CombineOp::And,
            filters,
        });
        assert!(!combine.is_below_depth(1));
        assert!(combine.is_below_depth(2));

        let filters = Filters(vec![compare, combine_1]);
        assert!(!filters.is_below_depth(1));
        assert!(filters.is_below_depth(2));
        let combine = Filter::Combine(Combine {
            operation: CombineOp::And,
            filters,
        });
        assert!(!combine.is_below_depth(1));
        assert!(combine.is_below_depth(2));
    }

    #[test]
    fn test_combine() {
        let compare = Filter::Compare(Compare {
            operation: CompareOp::Eq,
            field: "prop".try_into().unwrap(),
            value: json!("test").try_into().unwrap(),
        });
        let combine_0 = Combine {
            operation: CombineOp::And,
            filters: Filters(Vec::new()),
        };
        assert_eq!(
            serde_json::from_str::<Combine>(r#"{ "$and": [] }"#).unwrap(),
            combine_0,
        );

        let combine_1 = Combine {
            operation: CombineOp::And,
            filters: Filters(vec![compare.clone()]),
        };
        assert_eq!(
            serde_json::from_str::<Combine>(r#"{ "$and": [ { "prop": { "$eq": "test" } } ] }"#)
                .unwrap(),
            combine_1,
        );

        let combine_0 = Combine {
            operation: CombineOp::And,
            filters: Filters(vec![compare.clone(), Filter::Combine(combine_0)]),
        };
        assert_eq!(
            serde_json::from_str::<Combine>(
                r#"{ "$and": [ { "prop": { "$eq": "test" } }, { "$and": [] } ] }"#
            )
            .unwrap(),
            combine_0,
        );

        let combine_1 = Combine {
            operation: CombineOp::And,
            filters: Filters(vec![compare, Filter::Combine(combine_1)]),
        };
        assert_eq!(
            serde_json::from_str::<Combine>(
                r#"{ "$and": [
                    { "prop": { "$eq": "test" } },
                    { "$and": [ { "prop": { "$eq": "test" } } ] }
                ] }"#
            )
            .unwrap(),
            combine_1,
        );

        assert_invalid_len::<Combine>("{}", 0, 2);
        assert_invalid_len::<Combine>(r#"{ "$and": [], "$and": [] }"#, 2, 26);
        assert_eq!(
            serde_json::from_str::<Combine>(r#"{ "$and": [ { "$and": [], "$and": [] } ] }"#)
                .unwrap_err()
                .to_string(),
                format!(
                    "invalid variant, expected one of: Compare(invalid length 2, expected {}); Combine(invalid length 2, expected {}); Ids(missing field `$ids`) at line 1 column 40",
                    Compare::EXPECTING,
                    Combine::EXPECTING,
                ),
        );
        assert_eq!(
            serde_json::from_str::<Combine>(r#"{ "$and": [ { "$and": [ { "$and": [] } ] } ] }"#)
                .unwrap_err()
                .to_string(),
                format!(
                    "invalid value: more than 2 times nested combinations, expected {} at line 1 column 46",
                    Combine::EXPECTING,
                ),
        );
        let filters =
            vec![json!({ "prop": { "$eq": "test" } }); Filters::MAX_FILTERS_PER_COMBINATION + 1];
        assert_eq!(
            serde_json::from_str::<Combine>(&json!({ "$and": filters }).to_string())
                .unwrap_err()
                .to_string(),
            format!(
                "invalid length {}, expected {} at line 1 column 273",
                Filters::MAX_FILTERS_PER_COMBINATION + 1,
                Filters::EXPECTING,
            ),
        );
    }

    #[test]
    fn test_filter() {
        assert_eq!(
            serde_json::from_str::<Filter>(
                r#"{ "$and": [
                    { "$or": [ { "p1": { "$eq": "a" } }, { "p2": { "$in": ["b", "c"] } } ] },
                    { "p3": { "$gt": "1234-05-06T07:08:09Z" } },
                    { "p4": { "$lt": 42 } }
                ] }"#
            )
            .unwrap(),
            Filter::Combine(Combine {
                operation: CombineOp::And,
                filters: Filters(vec![
                    Filter::Combine(Combine {
                        operation: CombineOp::Or,
                        filters: Filters(vec![
                            Filter::Compare(Compare {
                                operation: CompareOp::Eq,
                                field: "p1".try_into().unwrap(),
                                value: json!("a").try_into().unwrap()
                            }),
                            Filter::Compare(Compare {
                                operation: CompareOp::In,
                                field: "p2".try_into().unwrap(),
                                value: json!(["b", "c"]).try_into().unwrap()
                            })
                        ])
                    }),
                    Filter::Compare(Compare {
                        operation: CompareOp::Gt,
                        field: "p3".try_into().unwrap(),
                        value: json!(DATE).try_into().unwrap()
                    }),
                    Filter::Compare(Compare {
                        operation: CompareOp::Lt,
                        field: "p4".try_into().unwrap(),
                        value: json!(42).try_into().unwrap()
                    })
                ])
            }),
        );
    }

    #[test]
    fn test_insert_published_after() {
        assert!(Filter::insert_published_after(None, None).is_none());

        let published_after_date =
            DateTime::<Utc>::from(DateTime::parse_from_rfc3339(DATE).unwrap());
        let published_after_filter = Filter::Compare(Compare {
            operation: CompareOp::Gte,
            field: "publication_date".try_into().unwrap(),
            value: json!(published_after_date.to_rfc3339()).try_into().unwrap(),
        });
        assert_eq!(
            Filter::insert_published_after(None, Some(published_after_date)).unwrap(),
            published_after_filter,
        );

        let compare = Filter::Compare(Compare {
            operation: CompareOp::Eq,
            field: "prop".try_into().unwrap(),
            value: json!("test").try_into().unwrap(),
        });
        assert_eq!(
            Filter::insert_published_after(Some(compare.clone()), None).unwrap(),
            compare,
        );

        let combine_and = Filter::Combine(Combine {
            operation: CombineOp::And,
            filters: Filters(vec![compare.clone(), published_after_filter.clone()]),
        });
        assert_eq!(
            Filter::insert_published_after(Some(compare.clone()), Some(published_after_date))
                .unwrap(),
            combine_and,
        );
        assert_eq!(
            Filter::insert_published_after(
                Some(Filter::Combine(Combine {
                    operation: CombineOp::And,
                    filters: Filters(vec![compare.clone()]),
                })),
                Some(published_after_date),
            )
            .unwrap(),
            combine_and,
        );

        let combine_or = Filter::Combine(Combine {
            operation: CombineOp::Or,
            filters: Filters(vec![compare]),
        });
        assert_eq!(
            Filter::insert_published_after(Some(combine_or.clone()), Some(published_after_date))
                .unwrap(),
            Filter::Combine(Combine {
                operation: CombineOp::And,
                filters: Filters(vec![combine_or, published_after_filter]),
            }),
        );
    }

    #[test]
    fn test_validate_unindexed() {
        let id = DocumentPropertyId::try_from("p").unwrap();
        let schema = IndexedPropertiesSchema::from(HashMap::new());
        let assert_unindexed = |op: &str, value: Value| {
            let filter =
                serde_json::from_str::<Filter>(&json!({ id.as_str(): { op: value } }).to_string())
                    .unwrap();
            let error = InvalidDocumentProperty {
                property_id: id.clone(),
                invalid_value: value,
                invalid_reason: InvalidDocumentPropertyReason::UnindexedId,
            };
            assert_eq!(filter.validate(&schema).unwrap_err(), error);
        };
        assert_unindexed("$eq", json!(true));
        assert_unindexed("$eq", json!("abc"));
        assert_unindexed("$in", json!(["a", "b", "c"]));
        assert_unindexed("$gt", json!(42));
        assert_unindexed("$gt", json!(DATE));
    }

    #[allow(clippy::needless_pass_by_value)]
    fn assert_compatible(
        id: &DocumentPropertyId,
        op: &str,
        value: Value,
        expected: IndexedPropertyType,
    ) {
        let filter =
            serde_json::from_str::<Filter>(&json!({ id.as_str(): { op: value } }).to_string())
                .unwrap();
        let schema =
            HashMap::from([(id.clone(), IndexedPropertyDefinition { r#type: expected })]).into();
        filter.validate(&schema).unwrap();
    }

    fn assert_incompatible(
        id: &DocumentPropertyId,
        op: &str,
        value: Value,
        expected: IndexedPropertyType,
    ) {
        let filter =
            serde_json::from_str::<Filter>(&json!({ id.as_str(): { op: value } }).to_string())
                .unwrap();
        let schema =
            HashMap::from([(id.clone(), IndexedPropertyDefinition { r#type: expected })]).into();
        let error = InvalidDocumentProperty {
            property_id: id.clone(),
            invalid_value: value,
            invalid_reason: InvalidDocumentPropertyReason::IncompatibleType { expected },
        };
        assert_eq!(filter.validate(&schema).unwrap_err(), error);
    }

    #[test]
    fn test_validate_bool() {
        let id = DocumentPropertyId::try_from("p").unwrap();
        let expected = IndexedPropertyType::Boolean;
        assert_compatible(&id, "$eq", json!(true), expected);
        assert_incompatible(&id, "$eq", json!("abc"), expected);
        assert_incompatible(&id, "$in", json!(["a", "b", "c"]), expected);
        assert_incompatible(&id, "$gt", json!(42), expected);
        assert_incompatible(&id, "$gt", json!(DATE), expected);
    }

    #[test]
    fn test_validate_string() {
        let id = DocumentPropertyId::try_from("p").unwrap();
        let expected = IndexedPropertyType::Keyword;
        assert_compatible(&id, "$eq", json!("abc"), expected);
        assert_compatible(&id, "$in", json!(["a", "b", "c"]), expected);
        assert_compatible(&id, "$eq", json!(DATE), expected);
        assert_incompatible(&id, "$eq", json!(true), expected);
        assert_incompatible(&id, "$gt", json!(42), expected);
    }

    #[test]
    fn test_validate_array_string() {
        let id = DocumentPropertyId::try_from("p").unwrap();
        let expected = IndexedPropertyType::KeywordArray;
        assert_compatible(&id, "$in", json!(["a", "b", "c"]), expected);
        assert_incompatible(&id, "$eq", json!(true), expected);
        assert_incompatible(&id, "$eq", json!("abc"), expected);
        assert_incompatible(&id, "$gt", json!(42), expected);
        assert_incompatible(&id, "$eq", json!(DATE), expected);
    }

    #[test]
    fn test_validate_number() {
        let id = DocumentPropertyId::try_from("p").unwrap();
        let expected = IndexedPropertyType::Number;
        assert_compatible(&id, "$gt", json!(42), expected);
        assert_incompatible(&id, "$eq", json!(true), expected);
        assert_incompatible(&id, "$eq", json!("abc"), expected);
        assert_incompatible(&id, "$in", json!(["a", "b", "c"]), expected);
        assert_incompatible(&id, "$eq", json!(DATE), expected);
    }

    #[test]
    fn test_validate_date() {
        let id = DocumentPropertyId::try_from("p").unwrap();
        let expected = IndexedPropertyType::Date;
        assert_compatible(&id, "$eq", json!(DATE), expected);
        assert_compatible(&id, "$gt", json!(DATE), expected);
        assert_incompatible(&id, "$eq", json!(true), expected);
        assert_incompatible(&id, "$in", json!(["a", "b", "c"]), expected);
        assert_incompatible(&id, "$gt", json!(42), expected);

        let value = json!("abc");
        let filter =
            serde_json::from_str::<Filter>(&json!({ id.as_str(): { "$eq": value } }).to_string())
                .unwrap();
        let schema =
            HashMap::from([(id.clone(), IndexedPropertyDefinition { r#type: expected })]).into();
        let error = InvalidDocumentProperty {
            property_id: id,
            invalid_value: value,
            invalid_reason: InvalidDocumentPropertyReason::MalformedDateTimeString,
        };
        assert_eq!(filter.validate(&schema).unwrap_err(), error);
    }

    #[test]
    fn test_ids_filter_can_be_provided() {
        let filter = serde_json::from_value::<Filter>(json!({
            "$ids": ["foo", "bar", "baz"]
        }))
        .unwrap();

        assert_eq!(
            filter,
            Filter::Ids(Ids {
                ids: vec![
                    "foo".parse().unwrap(),
                    "bar".parse().unwrap(),
                    "baz".parse().unwrap(),
                ]
            })
        );

        serde_json::from_value::<Filter>(json!({
            "$ids": ["$$$$"]
        }))
        .unwrap_err();

        serde_json::from_value::<Filter>(json!({
            "$ids": []
        }))
        .unwrap();

        serde_json::from_value::<Filter>(json!({
            "$ids": vec!["foo"; CompareOp::MAX_VALUES_PER_IN]
        }))
        .unwrap();

        serde_json::from_value::<Filter>(json!({
            "$ids": vec!["foo"; CompareOp::MAX_VALUES_PER_IN + 1]
        }))
        .unwrap_err();
    }
}
