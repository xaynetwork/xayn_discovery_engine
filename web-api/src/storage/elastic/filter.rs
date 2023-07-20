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

use std::{cmp::Ordering, collections::HashMap};

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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum GreaterRangeOp {
    Gte,
    Gt,
}

#[derive(Debug, PartialEq)]
struct GreaterRange<'a> {
    operation: GreaterRangeOp,
    value: &'a DocumentProperty,
}

impl GreaterRange<'_> {
    fn and(&mut self, other: Self) {
        match self.value.partial_cmp(other.value) {
            Some(Ordering::Less) => *self = other,
            Some(Ordering::Equal) if self.operation < other.operation => {
                self.operation = other.operation;
            }
            _ => {}
        }
    }

    fn or(&mut self, other: Self) {
        match self.value.partial_cmp(other.value) {
            Some(Ordering::Greater) => *self = other,
            Some(Ordering::Equal) if self.operation > other.operation => {
                self.operation = other.operation;
            }
            _ => {}
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum LessRangeOp {
    Lt,
    Lte,
}

#[derive(Debug)]
struct LessRange<'a> {
    operation: LessRangeOp,
    value: &'a DocumentProperty,
}

impl LessRange<'_> {
    fn and(&mut self, other: Self) {
        match self.value.partial_cmp(other.value) {
            Some(Ordering::Greater) => *self = other,
            Some(Ordering::Equal) if self.operation > other.operation => {
                self.operation = other.operation;
            }
            _ => {}
        }
    }

    fn or(&mut self, other: Self) {
        match self.value.partial_cmp(other.value) {
            Some(Ordering::Less) => *self = other,
            Some(Ordering::Equal) if self.operation < other.operation => {
                self.operation = other.operation;
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
struct Range<'a> {
    field: &'a DocumentPropertyId,
    // at least one of the options is Some(_)
    greater: Option<GreaterRange<'a>>,
    less: Option<LessRange<'a>>,
}

impl Serialize for Range<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Eq, Hash, PartialEq, Serialize)]
        #[serde(untagged)]
        enum RangeOp {
            Greater(GreaterRangeOp),
            Less(LessRangeOp),
        }

        #[derive(Serialize)]
        struct Range<'a> {
            range: HashMap<&'a str, HashMap<RangeOp, &'a DocumentProperty>>,
        }

        let field = format!("properties.{}", self.field);
        let mut range = HashMap::with_capacity(2);
        if let Some(GreaterRange { operation, value }) = self.greater {
            range.insert(RangeOp::Greater(operation), value);
        }
        if let Some(LessRange { operation, value }) = self.less {
            range.insert(RangeOp::Less(operation), value);
        }
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
            #[serde(rename = "bool")]
            filter: Filter<'a>,
        }

        let filter = Filter {
            filter: &self.filter,
        };

        if self.is_root {
            filter.serialize(serializer)
        } else {
            SubFilter { filter }.serialize(serializer)
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
            #[serde(rename = "bool")]
            should: Should<'a>,
        }

        let should = Should {
            should: &self.should,
            minimum_should_match: 1,
        };

        if self.is_root {
            should.serialize(serializer)
        } else {
            SubShould { should }.serialize(serializer)
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
            #[serde(rename = "bool")]
            must_not: MustNot<'a>,
        }

        let must_not = MustNot {
            must_not: &self.must_not,
        };

        if self.is_root {
            must_not.serialize(serializer)
        } else {
            SubMustNot { must_not }.serialize(serializer)
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

fn merge_range_and(mut clause: Vec<Clause<'_>>) -> Vec<Clause<'_>> {
    fn is_range<'a>(clause: &Clause<'a>) -> Option<&'a DocumentPropertyId> {
        if let Clause::Range(range) = clause {
            Some(range.field)
        } else {
            None
        }
    }

    let mut i = 0;
    while i < clause.len() {
        if let Some(field) = is_range(&clause[i]).map(ToOwned::to_owned) {
            let mut j = i + 1;
            while j < clause.len() {
                if is_range(&clause[j]).map_or(false, |f| f == &field) {
                    let Clause::Range(range_j) = clause.swap_remove(j) else {
                        unreachable!(/* clause[j] is range */);
                    };
                    let Clause::Range(range_i) = &mut clause[i] else {
                        unreachable!(/* clause[i] is range */);
                    };
                    match (&mut range_i.greater, range_j.greater) {
                        (Some(greater_i), Some(greater_j)) => greater_i.and(greater_j),
                        (None, greater_j @ Some(_)) => range_i.greater = greater_j,
                        (Some(_) | None, None) => {}
                    }
                    match (&mut range_i.less, range_j.less) {
                        (Some(less_i), Some(less_j)) => less_i.and(less_j),
                        (None, less_j @ Some(_)) => range_i.less = less_j,
                        (Some(_) | None, None) => {}
                    }
                } else {
                    j += 1;
                }
            }
        }
        i += 1;
    }

    clause
}

fn merge_range_or(mut clause: Vec<Clause<'_>>) -> Vec<Clause<'_>> {
    fn is_greater_range<'a>(clause: &Clause<'a>) -> Option<&'a DocumentPropertyId> {
        if let Clause::Range(range) = clause {
            range.greater.is_some().then_some(range.field)
        } else {
            None
        }
    }

    fn is_less_range<'a>(clause: &Clause<'a>) -> Option<&'a DocumentPropertyId> {
        if let Clause::Range(range) = clause {
            range.less.is_some().then_some(range.field)
        } else {
            None
        }
    }

    let mut i = 0;
    while i < clause.len() {
        if let Some(field) = is_greater_range(&clause[i]).map(ToOwned::to_owned) {
            let mut j = i + 1;
            while j < clause.len() {
                if is_greater_range(&clause[j]).map_or(false, |f| f == &field) {
                    let Clause::Range(Range { greater: Some(greater_j), .. }) =
                        clause.swap_remove(j)
                    else {
                        unreachable!(/* clause[j] is greater range */);
                    };
                    let Clause::Range(Range { greater: Some(greater_i), .. }) =
                        &mut clause[i]
                    else {
                        unreachable!(/* clause[i] is greater range */);
                    };
                    greater_i.or(greater_j);
                } else {
                    j += 1;
                }
            }
        } else if let Some(field) = is_less_range(&clause[i]).map(ToOwned::to_owned) {
            let mut j = i + 1;
            while j < clause.len() {
                if is_less_range(&clause[j]).map_or(false, |f| f == &field) {
                    let Clause::Range(Range { less: Some(less_j), .. }) =
                        clause.swap_remove(j)
                    else {
                        unreachable!(/* clause[j] is less range */);
                    };
                    let Clause::Range(Range { less: Some(less_i), .. }) = &mut clause[i] else {
                        unreachable!(/* clause[i] is less range */);
                    };
                    less_i.or(less_j);
                } else {
                    j += 1;
                }
            }
        }
        i += 1;
    }

    clause
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
                        field,
                        greater: Some(GreaterRange {
                            operation: GreaterRangeOp::Gt,
                            value,
                        }),
                        less: None,
                    }),
                    CompareOp::Gte => Self::Range(Range {
                        field,
                        greater: Some(GreaterRange {
                            operation: GreaterRangeOp::Gte,
                            value,
                        }),
                        less: None,
                    }),
                    CompareOp::Lt => Self::Range(Range {
                        field,
                        greater: None,
                        less: Some(LessRange {
                            operation: LessRangeOp::Lt,
                            value,
                        }),
                    }),
                    CompareOp::Lte => Self::Range(Range {
                        field,
                        greater: None,
                        less: Some(LessRange {
                            operation: LessRangeOp::Lte,
                            value,
                        }),
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
                        filter: merge_range_and(clause),
                        is_root,
                    }),
                    CombineOp::Or => Self::Should(Should {
                        should: merge_range_or(clause),
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
        let clause = serde_json::from_str(r#"{ "a": { "$eq": true } }"#).unwrap();
        let value = json!({ FILTER: [{ TERM: { PROP_A: true } }] });
        assert_eq!(
            serde_json::to_value(Clause::new(&clause, true)).unwrap(),
            value,
        );

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
    fn test_greater_range_and() {
        let val_1 = &json!(21).try_into().unwrap();
        let val_2 = &json!(42).try_into().unwrap();
        for (op_1, op_2) in [
            (GreaterRangeOp::Gt, GreaterRangeOp::Gt),
            (GreaterRangeOp::Gt, GreaterRangeOp::Gte),
            (GreaterRangeOp::Gte, GreaterRangeOp::Gt),
            (GreaterRangeOp::Gte, GreaterRangeOp::Gte),
        ] {
            let mut range = GreaterRange {
                operation: op_1,
                value: val_1,
            };
            range.and(GreaterRange {
                operation: op_2,
                value: val_2,
            });
            assert_eq!(range.operation, op_2);
            assert_eq!(range.value, val_2);

            range.and(GreaterRange {
                operation: op_1,
                value: val_1,
            });
            assert_eq!(range.operation, op_2);
            assert_eq!(range.value, val_2);

            range.and(GreaterRange {
                operation: op_1,
                value: val_2,
            });
            if op_2 == GreaterRangeOp::Gte && op_1 == GreaterRangeOp::Gt {
                assert_eq!(range.operation, op_1);
            } else {
                assert_eq!(range.operation, op_2);
            }
            assert_eq!(range.value, val_2);
        }
    }

    #[test]
    fn test_greater_range_or() {
        let val_1 = &json!(42).try_into().unwrap();
        let val_2 = &json!(21).try_into().unwrap();
        for (op_1, op_2) in [
            (GreaterRangeOp::Gt, GreaterRangeOp::Gt),
            (GreaterRangeOp::Gt, GreaterRangeOp::Gte),
            (GreaterRangeOp::Gte, GreaterRangeOp::Gt),
            (GreaterRangeOp::Gte, GreaterRangeOp::Gte),
        ] {
            let mut range = GreaterRange {
                operation: op_1,
                value: val_1,
            };
            range.or(GreaterRange {
                operation: op_2,
                value: val_2,
            });
            assert_eq!(range.operation, op_2);
            assert_eq!(range.value, val_2);

            range.or(GreaterRange {
                operation: op_1,
                value: val_1,
            });
            assert_eq!(range.operation, op_2);
            assert_eq!(range.value, val_2);

            range.or(GreaterRange {
                operation: op_1,
                value: val_2,
            });
            if op_2 == GreaterRangeOp::Gt && op_1 == GreaterRangeOp::Gte {
                assert_eq!(range.operation, op_1);
            } else {
                assert_eq!(range.operation, op_2);
            }
            assert_eq!(range.value, val_2);
        }
    }

    #[test]
    fn test_less_range_and() {
        let val_1 = &json!(42).try_into().unwrap();
        let val_2 = &json!(21).try_into().unwrap();
        for (op_1, op_2) in [
            (LessRangeOp::Lt, LessRangeOp::Lt),
            (LessRangeOp::Lt, LessRangeOp::Lte),
            (LessRangeOp::Lte, LessRangeOp::Lt),
            (LessRangeOp::Lte, LessRangeOp::Lte),
        ] {
            let mut range = LessRange {
                operation: op_1,
                value: val_1,
            };
            range.and(LessRange {
                operation: op_2,
                value: val_2,
            });
            assert_eq!(range.operation, op_2);
            assert_eq!(range.value, val_2);

            range.and(LessRange {
                operation: op_1,
                value: val_1,
            });
            assert_eq!(range.operation, op_2);
            assert_eq!(range.value, val_2);

            range.and(LessRange {
                operation: op_1,
                value: val_2,
            });
            if op_2 == LessRangeOp::Lte && op_1 == LessRangeOp::Lt {
                assert_eq!(range.operation, op_1);
            } else {
                assert_eq!(range.operation, op_2);
            }
            assert_eq!(range.value, val_2);
        }
    }

    #[test]
    fn test_less_range_or() {
        let val_1 = &json!(21).try_into().unwrap();
        let val_2 = &json!(42).try_into().unwrap();
        for (op_1, op_2) in [
            (LessRangeOp::Lt, LessRangeOp::Lt),
            (LessRangeOp::Lt, LessRangeOp::Lte),
            (LessRangeOp::Lte, LessRangeOp::Lt),
            (LessRangeOp::Lte, LessRangeOp::Lte),
        ] {
            let mut range = LessRange {
                operation: op_1,
                value: val_1,
            };
            range.or(LessRange {
                operation: op_2,
                value: val_2,
            });
            assert_eq!(range.operation, op_2);
            assert_eq!(range.value, val_2);

            range.or(LessRange {
                operation: op_1,
                value: val_1,
            });
            assert_eq!(range.operation, op_2);
            assert_eq!(range.value, val_2);

            range.or(LessRange {
                operation: op_1,
                value: val_2,
            });
            if op_2 == LessRangeOp::Lt && op_1 == LessRangeOp::Lte {
                assert_eq!(range.operation, op_1);
            } else {
                assert_eq!(range.operation, op_2);
            }
            assert_eq!(range.value, val_2);
        }
    }

    #[test]
    fn test_range() {
        for operation in [
            ("$gt", "gt"),
            ("$gte", "gte"),
            ("$lt", "lt"),
            ("$lte", "lte"),
        ] {
            let clause =
                serde_json::from_str(&json!({ "a": { operation.0: 42 } }).to_string()).unwrap();
            let value = json!({ FILTER: [{ RANGE: { PROP_A: { operation.1: 42 } } }] });
            assert_eq!(
                serde_json::to_value(Clause::new(&clause, true)).unwrap(),
                value,
            );

            let clause =
                serde_json::from_str(&json!({ "a": { operation.0: DATE } }).to_string()).unwrap();
            let value = json!({ FILTER: [{ RANGE: { PROP_A: { operation.1: DATE } } }] });
            assert_eq!(
                serde_json::to_value(Clause::new(&clause, true)).unwrap(),
                value,
            );
        }
    }

    #[test]
    fn test_fully_merge_range() {
        let clause = serde_json::from_str(
            r#"{ "$and": [
                { "a": { "$gt": 3 } },
                { "c": { "$gt": 5 } },
                { "a": { "$gte": 4 } },
                { "a": { "$lte": 2 } },
                { "c": { "$lte": 0 } },
                { "a": { "$lt": 1 } }
            ] }"#,
        )
        .unwrap();
        let Clause::Filter(Filter { filter, .. }) = Clause::new(&clause, true) else {
            panic!();
        };
        let value = json!([
            { RANGE: { PROP_A: { "gte": 4, "lt": 1 } } },
            { RANGE: { PROP_C: { "gt": 5, "lte": 0 } } }
        ]);
        assert_eq!(serde_json::to_value(filter).unwrap(), value);
    }

    #[test]
    fn test_partly_merge_range() {
        let clause = serde_json::from_str(
            r#"{ "$or": [
                { "a": { "$gt": 3 } },
                { "c": { "$gt": 5 } },
                { "a": { "$gte": 4 } },
                { "a": { "$lte": 2 } },
                { "c": { "$lte": 0 } },
                { "a": { "$lt": 1 } }
            ] }"#,
        )
        .unwrap();
        let Clause::Should(Should { should, .. }) = Clause::new(&clause, true) else {
            panic!();
        };
        let value = json!([
            { RANGE: { PROP_A: { "gt": 3 } } },
            { RANGE: { PROP_C: { "gt": 5 } } },
            { RANGE: { PROP_A: { "lte": 2 } } },
            { RANGE: { PROP_C: { "lte": 0 } } }
        ]);
        assert_eq!(serde_json::to_value(should).unwrap(), value);
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
            { RANGE: { PROP_C: { "gt": DATE, "lt": DATE } } }
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
        let filter_clause = json_array!([
            { TERM: { PROP_A: "b" } },
            { RANGE: { PROP_C: { "gt": DATE, "lt": DATE } } }
        ]);
        let should_clause = json_array!([
            { TERM: { PROP_A: "b" } },
            { RANGE: { PROP_C: { "gt": DATE } } },
            { RANGE: { PROP_C: { "lt": DATE } } }
        ]);
        let filter = json!({ BOOL: { FILTER: filter_clause } });
        let should = json!({ BOOL: { SHOULD: should_clause, MINIMUM_SHOULD_MATCH: 1 } });

        let clause = serde_json::from_str(
            &json!({ "$and": [and, or, filters[0], filters[1], and, filters[2]] }).to_string(),
        )
        .unwrap();
        let value = json!({
            FILTER: [filter, should, filter_clause[0], filter_clause[1], filter]
        });
        assert_eq!(
            serde_json::to_value(Clause::new(&clause, true)).unwrap(),
            value,
        );

        let clause = serde_json::from_str(
            &json!({ "$or": [or, and, filters[0], filters[1], or, filters[2]] }).to_string(),
        )
        .unwrap();
        let value = json!({
            SHOULD: [should, filter, should_clause[0], should_clause[1], should, should_clause[2]],
            MINIMUM_SHOULD_MATCH: 1
        });
        assert_eq!(
            serde_json::to_value(Clause::new(&clause, true)).unwrap(),
            value,
        );
    }
}
