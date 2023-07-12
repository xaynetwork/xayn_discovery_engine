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

use std::collections::HashMap;

use serde::{Serialize, Serializer};
use serde_json::Value;
use xayn_web_api_shared::serde::{merge_json_objects, JsonObject};

use crate::{
    models::{DocumentId, DocumentProperty, DocumentPropertyId},
    personalization::filter::{self, Combine, CombineOp, Compare, CompareOp},
    storage::KnnSearchParams,
};

#[derive(Debug)]
struct Term<'a> {
    field: &'a DocumentPropertyId,
    value: &'a DocumentProperty,
}

impl Serialize for Term<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Term<'a> {
            term: HashMap<&'a str, &'a DocumentProperty>,
        }

        let field = format!("properties.{}", self.field);
        let term = [(field.as_str(), self.value)].into();

        Term { term }.serialize(serializer)
    }
}

#[derive(Debug)]
struct Terms<'a> {
    field: &'a DocumentPropertyId,
    value: &'a DocumentProperty,
}

impl Serialize for Terms<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Terms<'a> {
            terms: HashMap<&'a str, &'a DocumentProperty>,
        }

        let field = format!("properties.{}", self.field);
        let terms = [(field.as_str(), self.value)].into();

        Terms { terms }.serialize(serializer)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum RangeOp {
    Gt,
    Gte,
    Lt,
    Lte,
}

#[derive(Debug)]
struct Range<'a> {
    operation: RangeOp,
    field: &'a DocumentPropertyId,
    value: &'a DocumentProperty,
}

impl Serialize for Range<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Range<'a> {
            range: HashMap<&'a str, HashMap<RangeOp, &'a DocumentProperty>>,
        }

        let field = format!("properties.{}", self.field);
        let range = [(self.operation, self.value)].into();
        let range = [(field.as_str(), range)].into();

        Range { range }.serialize(serializer)
    }
}

#[derive(Debug)]
struct Ids<'a> {
    values: &'a [DocumentId],
}

impl Serialize for Ids<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Values<'a> {
            values: &'a [DocumentId],
        }

        #[derive(Serialize)]
        struct Ids<'a> {
            ids: Values<'a>,
        }

        Ids {
            ids: Values {
                values: self.values,
            },
        }
        .serialize(serializer)
    }
}

#[derive(Debug)]
struct Filter<'a> {
    filter: Vec<Clause<'a>>,
    is_root: bool,
}

impl Serialize for Filter<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Filter<'a> {
            filter: &'a [Clause<'a>],
        }

        #[derive(Serialize)]
        struct SubFilter<'a> {
            bool: Filter<'a>,
        }

        let filter = Filter {
            filter: &self.filter,
        };

        if self.is_root {
            filter.serialize(serializer)
        } else {
            SubFilter { bool: filter }.serialize(serializer)
        }
    }
}

#[derive(Debug)]
struct Should<'a> {
    should: Vec<Clause<'a>>,
    is_root: bool,
}

impl Serialize for Should<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Should<'a> {
            should: &'a [Clause<'a>],
            minimum_should_match: usize,
        }

        #[derive(Serialize)]
        struct SubShould<'a> {
            bool: Should<'a>,
        }

        let should = Should {
            should: &self.should,
            minimum_should_match: 1,
        };

        if self.is_root {
            should.serialize(serializer)
        } else {
            SubShould { bool: should }.serialize(serializer)
        }
    }
}

#[derive(Debug)]
struct MustNot<'a> {
    must_not: Vec<Clause<'a>>,
    is_root: bool,
}

impl Serialize for MustNot<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct MustNot<'a> {
            must_not: &'a [Clause<'a>],
        }

        #[derive(Serialize)]
        struct SubMustNot<'a> {
            bool: MustNot<'a>,
        }

        let must_not = MustNot {
            must_not: &self.must_not,
        };

        if self.is_root {
            must_not.serialize(serializer)
        } else {
            SubMustNot { bool: must_not }.serialize(serializer)
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Clause<'a> {
    Term(Term<'a>),
    Terms(Terms<'a>),
    Range(Range<'a>),
    Ids(Ids<'a>),
    Filter(Filter<'a>),
    Should(Should<'a>),
    MustNot(MustNot<'a>),
}

impl<'a> Clause<'a> {
    fn new(clause: &'a filter::Filter, is_root: bool) -> Self {
        match clause {
            filter::Filter::Compare(Compare {
                operation,
                field,
                value,
            }) => {
                let clause = match operation {
                    CompareOp::Eq => Self::Term(Term { field, value }),
                    CompareOp::In => Self::Terms(Terms { field, value }),
                    CompareOp::Gt => Self::Range(Range {
                        operation: RangeOp::Gt,
                        field,
                        value,
                    }),
                    CompareOp::Gte => Self::Range(Range {
                        operation: RangeOp::Gte,
                        field,
                        value,
                    }),
                    CompareOp::Lt => Self::Range(Range {
                        operation: RangeOp::Lt,
                        field,
                        value,
                    }),
                    CompareOp::Lte => Self::Range(Range {
                        operation: RangeOp::Lte,
                        field,
                        value,
                    }),
                };

                if is_root {
                    Self::Filter(Filter {
                        filter: vec![clause],
                        is_root,
                    })
                } else {
                    clause
                }
            }

            filter::Filter::Combine(Combine { operation, filters }) => {
                let clause = filters
                    .iter()
                    .map(|clause| Self::new(clause, false))
                    .collect();

                match operation {
                    CombineOp::And => Self::Filter(Filter {
                        filter: clause,
                        is_root,
                    }),
                    CombineOp::Or => Self::Should(Should {
                        should: clause,
                        is_root,
                    }),
                }
            }
        }
    }

    fn excluded_ids(values: &'a [DocumentId]) -> Self {
        Self::MustNot(MustNot {
            must_not: vec![Clause::Ids(Ids { values })],
            is_root: true,
        })
    }
}

impl KnnSearchParams<'_> {
    pub(super) fn create_search_filter(&self) -> JsonObject {
        let mut clauses = Vec::new();
        if !self.excluded.is_empty() {
            // existing pg documents are not filtered in the query to avoid too much work for a cold
            // path, filtering them afterwards can occasionally lead to less than k results though
            clauses.push(Clause::excluded_ids(self.excluded));
        }
        if let Some(filter) = self.filter {
            clauses.push(Clause::new(filter, true));
        }

        merge_json_objects(clauses.into_iter().map(|clause| {
            let Ok(Value::Object(clause)) = serde_json::to_value(clause) else {
                unreachable!(
                    // clause serialization can't fail
                    // clause doesn't contain map with non-string keys
                    // clause is json object
                );
            };
            clause
        }))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use xayn_web_api_shared::serde::json_array;

    use super::*;

    const BOOL: &str = "bool";
    const DATE: &str = "1234-05-06T07:08:09Z";
    const FILTER: &str = "filter";
    const MINIMUM_SHOULD_MATCH: &str = "minimum_should_match";
    const PROP_A: &str = "properties.a";
    const PROP_C: &str = "properties.c";
    const RANGE: &str = "range";
    const SHOULD: &str = "should";
    const TERM: &str = "term";
    const TERMS: &str = "terms";

    #[test]
    fn test_term() {
        let clause = serde_json::from_str(r#"{ "a": { "$eq": "b" } }"#).unwrap();
        let value = json!({ FILTER: [{ TERM: { PROP_A: "b" } }] });
        assert_eq!(
            serde_json::to_value(Clause::new(&clause, true)).unwrap(),
            value,
        );
    }

    #[test]
    fn test_terms() {
        let clause = serde_json::from_str(r#"{ "a": { "$in": ["b", "c"] } }"#).unwrap();
        let value = json!({ FILTER: [{ TERMS: { PROP_A: ["b", "c"] } }] });
        assert_eq!(
            serde_json::to_value(Clause::new(&clause, true)).unwrap(),
            value,
        );
    }

    #[test]
    fn test_range() {
        for (clause, value) in [
            (
                &serde_json::from_str(&json!({ "a": { "$gt": DATE } }).to_string()).unwrap(),
                json!({ FILTER: [{ RANGE: { PROP_A: { "gt": DATE } } }] }),
            ),
            (
                &serde_json::from_str(&json!({ "a": { "$gte": DATE } }).to_string()).unwrap(),
                json!({ FILTER: [{ RANGE: { PROP_A: { "gte": DATE } } }] }),
            ),
            (
                &serde_json::from_str(&json!({ "a": { "$lt": DATE } }).to_string()).unwrap(),
                json!({ FILTER: [{ RANGE: { PROP_A: { "lt": DATE } } }] }),
            ),
            (
                &serde_json::from_str(&json!({ "a": { "$lte": DATE } }).to_string()).unwrap(),
                json!({ FILTER: [{ RANGE: { PROP_A: { "lte": DATE } } }] }),
            ),
        ] {
            assert_eq!(
                serde_json::to_value(Clause::new(clause, true)).unwrap(),
                value,
            );
        }
    }

    #[test]
    fn test_excluded_ids() {
        let ids = [
            "a".try_into().unwrap(),
            "b".try_into().unwrap(),
            "c".try_into().unwrap(),
        ];
        let clause = Clause::excluded_ids(&ids);
        let value = json!({ "must_not": [{ "ids": { "values": ["a", "b", "c"] } }] });
        assert_eq!(serde_json::to_value(clause).unwrap(), value);
    }

    #[test]
    fn test_filter() {
        let clause = serde_json::from_str(
            &json!({ "$and": [
                { "a": { "$eq": "b" } },
                { "c": { "$gt": DATE } },
                { "c": { "$lt": DATE } }
            ] })
            .to_string(),
        )
        .unwrap();
        let value = json!({ FILTER: [
            { TERM: { PROP_A: "b" } },
            { RANGE: { PROP_C: { "gt": DATE } } },
            { RANGE: { PROP_C: { "lt": DATE } } }
        ] });
        assert_eq!(
            serde_json::to_value(Clause::new(&clause, true)).unwrap(),
            value,
        );
    }

    #[test]
    fn test_should() {
        let clause = serde_json::from_str(
            &json!({ "$or": [
                { "a": { "$eq": "b" } },
                { "c": { "$gt": DATE } },
                { "c": { "$lt": DATE } }
            ] })
            .to_string(),
        )
        .unwrap();
        let value = json!({
            SHOULD: [
                { TERM: { PROP_A: "b" } },
                { RANGE: { PROP_C: { "gt": DATE } } },
                { RANGE: { PROP_C: { "lt": DATE } } }
            ],
            MINIMUM_SHOULD_MATCH: 1
        });
        assert_eq!(
            serde_json::to_value(Clause::new(&clause, true)).unwrap(),
            value,
        );
    }

    #[test]
    fn test_nested() {
        let filters = json_array!([
            { "a": { "$eq": "b" } },
            { "c": { "$gt": DATE } },
            { "c": { "$lt": DATE } }
        ]);
        let and = json!({ "$and": filters });
        let or = json!({ "$or": filters });
        let clauses = json_array!([
            { TERM: { PROP_A: "b" } },
            { RANGE: { PROP_C: { "gt": DATE } } },
            { RANGE: { PROP_C: { "lt": DATE } } }
        ]);
        let filter = json!({ BOOL: { FILTER: clauses } });
        let should = json!({ BOOL: { SHOULD: clauses, MINIMUM_SHOULD_MATCH: 1 } });

        let clause = serde_json::from_str(
            &json!({ "$and": [and, or, filters[0], and, filters[1], filters[2]] }).to_string(),
        )
        .unwrap();
        let value = json!({
            FILTER: [filter, should, clauses[0], filter, clauses[1], clauses[2]]
        });
        assert_eq!(
            serde_json::to_value(Clause::new(&clause, true)).unwrap(),
            value,
        );

        let clause = serde_json::from_str(
            &json!({ "$or": [or, and, filters[0], or, filters[1], filters[2]] }).to_string(),
        )
        .unwrap();
        let value = json!({
            SHOULD: [should, filter, clauses[0], should, clauses[1], clauses[2]],
            MINIMUM_SHOULD_MATCH: 1
        });
        assert_eq!(
            serde_json::to_value(Clause::new(&clause, true)).unwrap(),
            value,
        );
    }
}
