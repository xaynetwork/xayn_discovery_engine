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

use itertools::Itertools;
use serde_json::{json, Value};
use xayn_web_api_shared::serde::{JsonObject, json_array};

use crate::{
    personalization::filter::{CombineOp, CompareOp, Filter},
    storage::KnnSearchParams,
};

const BOOL: &str = "bool";
const FILTER: &str = "filter";
const MINIMUM_SHOULD_MATCH: &str = "minimum_should_match";
const RANGE: &str = "range";
const SHOULD: &str = "should";
const TERM: &str = "term";
const TERMS: &str = "terms";

fn merge_range(clause: &mut Vec<Value>) {
    fn is_range(clause: &Value) -> Option<&str> {
        clause
            .as_object()
            .and_then(|clause| clause.get(RANGE))
            .and_then(|range| range
                .as_object()
                .unwrap(/* clause[RANGE] is object */)
                .keys()
                .next(/* clause[RANGE] contains exactly one key */)
                .map(String::as_str)
            )
    }

    let mut i = 0;
    while i < clause.len() {
        if let Some(field) = is_range(&clause[i]).map(ToString::to_string) {
            let mut j = i + 1;
            while j < clause.len() {
                if is_range(&clause[j]).map_or(false, |f| f == field) {
                    let Value::Object(mut range) = clause.swap_remove(j) else {
                        unreachable!(/* clause[j] is object */);
                    };
                    let Value::Object(mut range) = range
                        .remove(RANGE)
                        .unwrap(/* clause[j] contains RANGE */)
                    else {
                        unreachable!(/* clause[j][RANGE] is object */);
                    };
                    let Value::Object(range) = range
                        .remove(&field)
                        .unwrap(/* clause[j][RANGE] contains field */) 
                    else {
                        unreachable!(/* clause[j][RANGE][field] is object */);
                    };
                    clause[i]
                        .as_object_mut()
                        .unwrap(/* clause[i] is object */)[RANGE]
                        .as_object_mut()
                        .unwrap(/* clause[i][RANGE] is object */)[&field]
                        .as_object_mut()
                        .unwrap(/* clause[i][RANGE][field] is object */)
                        .extend(range);
                } else {
                    j += 1;
                }
            }
        }
        i += 1;
    }
}

fn extend_value(filter: &mut JsonObject, occurence: &'static str, clause: Vec<Value>) {
    if let Some(filter) = filter.get_mut(occurence) {
        filter.as_array_mut().unwrap(/* filter[occurence] is array */).extend(clause);
    } else {
        filter.insert(occurence.to_string(), clause.into());
    }
    if occurence == FILTER {
        merge_range(filter[occurence].as_array_mut().unwrap(/* filter[occurence] is array */));
    }
}

fn extend_filter(filter: &mut JsonObject, clause: &Filter, is_not_root: Option<&'static str>) {
    match clause {
        Filter::Compare(compare) => {
            let field = format!("properties.{}", compare.field);
            let clause = match compare.operation {
                CompareOp::Eq => json_array!([{ TERM: { field: compare.value } }]),
                CompareOp::In => json_array!([{ TERMS: { field: compare.value } }]),
                CompareOp::Gt => json_array!([{ RANGE: { field: { "gt": compare.value } } }]),
                CompareOp::Gte => json_array!([{ RANGE: { field: { "gte": compare.value } } }]),
                CompareOp::Lt => json_array!([{ RANGE: { field: { "lt": compare.value } } }]),
                CompareOp::Lte => json_array!([{ RANGE: { field: { "lte": compare.value } } }]),
            };
            extend_value(filter, is_not_root.unwrap_or(FILTER), clause);
        }
        Filter::Combine(combine) => {
            let (occurence, clause) = match (combine.operation, is_not_root) {
                (CombineOp::And, _) => (FILTER, JsonObject::with_capacity(combine.filters.len())),
                (CombineOp::Or, Some(_)) => {
                    let mut clause = JsonObject::with_capacity(combine.filters.len() + 1);
                    clause.insert(MINIMUM_SHOULD_MATCH.to_string(), json!(1));
                    (SHOULD, clause)
                }
                (CombineOp::Or, None) => {
                    filter.insert(MINIMUM_SHOULD_MATCH.to_string(), json!(1));
                    (SHOULD, JsonObject::with_capacity(combine.filters.len()))
                }
            };
            let clause = combine.filters.iter().fold(clause, |mut filter, clause| {
                extend_filter(&mut filter, clause, Some(occurence));
                filter
            });
            let clause = if is_not_root.is_some() {
                json_array!([{ BOOL: clause }])
            } else {
                clause
                    .into_iter()
                    .flat_map(|(_, clause)| {
                        let Value::Array(clause) = clause else {
                            unreachable!(/* clause is array */);
                        };
                        clause
                    })
                    .collect_vec()
            };
            extend_value(filter, occurence, clause);
        }
    }
}

impl KnnSearchParams<'_> {
    pub(super) fn create_search_filter(&self) -> JsonObject {
        // filter clauses must be arrays to not break the assumptions of extend_filter()
        let mut filter = JsonObject::new();
        if !self.excluded.is_empty() {
            // existing pg documents are not filtered in the query to avoid too much work for a cold
            // path, filtering them afterwards can occasionally lead to less than k results though
            filter.insert(
                "must_not".to_string(),
                json!([{ "ids": { "values": self.excluded } }]),
            );
        }
        if let Some(opt_filter) = self.filter {
            extend_filter(&mut filter, opt_filter, None);
        }

        filter
    }
}

#[cfg(test)]
mod tests {
    use xayn_web_api_shared::serde::json_object;

    use super::*;

    const DATE: &str = "1234-05-06T07:08:09Z";

    #[test]
    fn test_merge_range() {
        let mut filter = Vec::new();
        merge_range(&mut filter);
        assert!(filter.is_empty());

        let mut filter = vec![json!({})];
        let expected = filter.clone();
        merge_range(&mut filter);
        assert_eq!(filter, expected);

        let mut filter =
            vec![json!({}), json!({}), json!({ RANGE: { "a": { "gt": "b" } } }), json!({})];
        let expected = filter.clone();
        merge_range(&mut filter);
        assert_eq!(filter, expected);

        let mut filter = vec![
            json!({ RANGE: { "a": { "gt": "b" } } }),
            json!({ RANGE: { "a": { "lt": "c" } } }),
            json!({ RANGE: { "a": { "gt": "d" } } }),
        ];
        merge_range(&mut filter);
        assert_eq!(
            filter,
            vec![json!({ RANGE: { "a": { "gt": "d", "lt": "c" } } })],
        );

        let mut filter = vec![
            json!({}),
            json!({ RANGE: { "a": { "gt": "b" } } }),
            json!({ RANGE: { "a": { "lt": "c" } } }),
            json!({}),
            json!({}),
            json!({ RANGE: { "a": { "gt": "d" } } }),
            json!({}),
        ];
        merge_range(&mut filter);
        assert_eq!(
            filter,
            vec![
                json!({}),
                json!({ RANGE: { "a": { "gt": "d", "lt": "c" } } }),
                json!({}),
                json!({}),
                json!({}),
            ],
        );

        let mut filter = vec![
            json!({ RANGE: { "a": { "gte": "b" } } }),
            json!({}),
            json!({ RANGE: { "c": { "lt": "d" } } }),
            json!({}),
            json!({}),
            json!({ RANGE: { "a": { "lte": "e" } } }),
            json!({ RANGE: { "c": { "gt": "f" } } }),
            json!({}),
        ];
        merge_range(&mut filter);
        assert_eq!(
            filter,
            vec![
                json!({ RANGE: { "a": { "gte": "b", "lte": "e" } } }),
                json!({}),
                json!({ RANGE: { "c": { "lt": "d", "gt": "f" } } }),
                json!({}),
                json!({}),
                json!({}),
            ],
        );
    }

    #[test]
    fn test_extend_value() {
        let mut filter = JsonObject::new();
        extend_value(&mut filter, FILTER, json_array!([]));
        assert_eq!(filter, json_object!({ FILTER: [] }));
        extend_value(&mut filter, FILTER, json_array!([{}]));
        assert_eq!(filter, json_object!({ FILTER: [{}] }));

        let mut filter = JsonObject::new();
        extend_value(&mut filter, FILTER, json_array!([{}]));
        assert_eq!(filter, json_object!({ FILTER: [{}] }));
        extend_value(&mut filter, FILTER, json_array!([{}, {}]));
        assert_eq!(filter, json_object!({ FILTER: [{}, {}, {}] }));

        let mut filter = json_object!({ SHOULD: [] });
        extend_value(&mut filter, FILTER, json_array!([{}]));
        assert_eq!(filter, json_object!({ SHOULD: [], FILTER: [{}] }));
    }

    #[test]
    fn test_extend_filter_compare_string() {
        for (clause, term) in [
            (
                &serde_json::from_str(r#"{ "a": { "$eq": "b" } }"#).unwrap(),
                json!({ TERM: { "properties.a": "b" } }),
            ),
            (
                &serde_json::from_str(r#"{ "a": { "$in": ["b", "c"] } }"#).unwrap(),
                json!({ TERMS: { "properties.a": ["b", "c"] } }),
            ),
        ] {
            let mut filter = JsonObject::new();
            extend_filter(&mut filter, clause, None);
            assert_eq!(filter, json_object!({ FILTER: [term] }));

            let mut filter = json_object!({ FILTER: [] });
            extend_filter(&mut filter, clause, None);
            assert_eq!(filter, json_object!({ FILTER: [term] }));

            let mut filter = json_object!({ FILTER: [{}] });
            extend_filter(&mut filter, clause, None);
            assert_eq!(filter, json_object!({ FILTER: [{}, term] }));

            let mut filter = json_object!({ SHOULD: [] });
            extend_filter(&mut filter, clause, None);
            assert_eq!(filter, json_object!({ SHOULD: [], FILTER: [term] }));
        }
    }

    

    #[test]
    fn test_filter_compare_date() {
        const FIELD: &str = "properties.a";
        for (clause, term) in [
            (
                &serde_json::from_str(r#"{ "a": { "$gt": "1234-05-06T07:08:09Z" } }"#).unwrap(),
                json_object!({ RANGE: { FIELD: { "gt": DATE } } }),
            ),
            (
                &serde_json::from_str(r#"{ "a": { "$gte": "1234-05-06T07:08:09Z" } }"#).unwrap(),
                json_object!({ RANGE: { FIELD: { "gte": DATE } } }),
            ),
            (
                &serde_json::from_str(r#"{ "a": { "$lt": "1234-05-06T07:08:09Z" } }"#).unwrap(),
                json_object!({ RANGE: { FIELD: { "lt": DATE } } }),
            ),
            (
                &serde_json::from_str(r#"{ "a": { "$lte": "1234-05-06T07:08:09Z" } }"#).unwrap(),
                json_object!({ RANGE: { FIELD: { "lte": DATE } } }),
            ),
        ] {
            let mut filter = JsonObject::new();
            extend_filter(&mut filter, clause, None);
            assert_eq!(filter, json_object!({ FILTER: [term] }));

            let mut filter = json_object!({ FILTER: [] });
            extend_filter(&mut filter, clause, None);
            assert_eq!(filter, json_object!({ FILTER: [term] }));

            let mut filter = json_object!({ FILTER: [{}] });
            extend_filter(&mut filter, clause, None);
            assert_eq!(filter, json_object!({ FILTER: [{}, term] }));

            let mut filter = json_object!({ SHOULD: [] });
            extend_filter(&mut filter, clause, None);
            assert_eq!(filter, json_object!({ SHOULD: [], FILTER: [term] }));
        }

        let mut filter = json_object!({ FILTER: [{ RANGE: { FIELD: { "gt": DATE } } }] });
        extend_filter(
            &mut filter,
            &serde_json::from_str(r#"{ "a": { "$lt": "1234-05-06T07:08:09Z" } }"#).unwrap(),
            None,
        );
        assert_eq!(
            filter,
            json_object!({ FILTER: [{ RANGE: { FIELD: { "gt": DATE, "lt": DATE } } }] }),
        );
    }

    #[test]
    fn test_extend_filter_combine_and() {
        let clause = &serde_json::from_str(r#"{ "$and": [
            { "a": { "$eq": "b" } },
            { "c": { "$gt": "1234-05-06T07:08:09Z" } },
            { "c": { "$lt": "1234-05-06T07:08:09Z" } }
        ] }"#)
        .unwrap();
        let term = json!({ TERM: { "properties.a": "b" } });
        let range = json_object!({ RANGE: { "properties.c": { "gt": DATE, "lt": DATE } } });

        let mut filter = JsonObject::new();
        extend_filter(&mut filter, clause, None);
        assert_eq!(filter, json_object!({ FILTER: [term, range] }));

        let mut filter = json_object!({ FILTER: [] });
        extend_filter(&mut filter, clause, None);
        assert_eq!(filter, json_object!({ FILTER: [term, range] }));

        let mut filter = json_object!({ FILTER: [{}] });
        extend_filter(&mut filter, clause, None);
        assert_eq!(filter, json_object!({ FILTER: [{}, term, range] }));

        let mut filter = json_object!({ SHOULD: [] });
        extend_filter(&mut filter, clause, None);
        assert_eq!(filter, json_object!({ SHOULD: [], FILTER: [term, range] }));
    }

    #[test]
    #[allow(clippy::similar_names)]
    fn test_extend_filter_combine_or() {
        let clause = &serde_json::from_str(r#"{ "$or": [
            { "a": { "$eq": "b" } },
            { "c": { "$gt": "1234-05-06T07:08:09Z" } },
            { "c": { "$lt": "1234-05-06T07:08:09Z" } }
        ] }"#)
        .unwrap();
        let term = json_object!({ TERM: { "properties.a": "b" } });
        let range_gt = json_object!({ RANGE: { "properties.c": { "gt": DATE } } });
        let range_lt = json_object!({ RANGE: { "properties.c": { "lt": DATE } } });

        let mut filter = JsonObject::new();
        extend_filter(&mut filter, clause, None);
        assert_eq!(
            filter,
            json_object!({ SHOULD: [term, range_gt, range_lt], MINIMUM_SHOULD_MATCH: 1 }),
        );

        let mut filter = json_object!({ SHOULD: [] });
        extend_filter(&mut filter, clause, None);
        assert_eq!(
            filter,
            json_object!({ SHOULD: [term, range_gt, range_lt], MINIMUM_SHOULD_MATCH: 1 }),
        );

        let mut filter = json_object!({ SHOULD: [{}] });
        extend_filter(&mut filter, clause, None);
        assert_eq!(
            filter,
            json_object!({ SHOULD: [{}, term, range_gt, range_lt], MINIMUM_SHOULD_MATCH: 1 }),
        );

        let mut filter = json_object!({ FILTER: [] });
        extend_filter(&mut filter, clause, None);
        assert_eq!(
            filter,
            json_object!({ FILTER: [], SHOULD: [term, range_gt, range_lt], MINIMUM_SHOULD_MATCH: 1 }),
        );
    }

    #[test]
    #[allow(clippy::similar_names)]
    fn test_extend_filter_nested() {
        let clause = &serde_json::from_str(r#"{ "$and": [
            { "$and": [
                { "a": { "$eq": "b" } },
                { "c": { "$gt": "1234-05-06T07:08:09Z" } },
                { "c": { "$lt": "1234-05-06T07:08:09Z" } }
            ] },
            { "$or": [
                { "a": { "$eq": "b" } },
                { "c": { "$gt": "1234-05-06T07:08:09Z" } },
                { "c": { "$lt": "1234-05-06T07:08:09Z" } }
            ] },
            { "$and": [
                { "a": { "$eq": "b" } },
                { "c": { "$gt": "1234-05-06T07:08:09Z" } },
                { "c": { "$lt": "1234-05-06T07:08:09Z" } }
            ] }
        ] }"#)
        .unwrap();
        let term = json_object!({ TERM: { "properties.a": "b" } });
        let range = json_object!({ RANGE: { "properties.c": { "gt": DATE, "lt": DATE } } });
        let range_gt = json_object!({ RANGE: { "properties.c": { "gt": DATE } } });
        let range_lt = json_object!({ RANGE: { "properties.c": { "lt": DATE } } });

        let mut filter = JsonObject::new();
        extend_filter(&mut filter, clause, None);
        assert_eq!(
            filter,
            json_object!({ FILTER: [
                { BOOL: { FILTER: [term, range] } },
                { BOOL: { FILTER: [term, range] } },
                { BOOL: { SHOULD: [term, range_gt, range_lt], MINIMUM_SHOULD_MATCH: 1 } }
            ] }),
        );

        let clause = &serde_json::from_str(r#"{ "$or": [
            { "$or": [
                { "a": { "$eq": "b" } },
                { "c": { "$gt": "1234-05-06T07:08:09Z" } },
                { "c": { "$lt": "1234-05-06T07:08:09Z" } }
            ] },
            { "$and": [
                { "a": { "$eq": "b" } },
                { "c": { "$gt": "1234-05-06T07:08:09Z" } },
                { "c": { "$lt": "1234-05-06T07:08:09Z" } }
            ] },
            { "$or": [
                { "a": { "$eq": "b" } },
                { "c": { "$gt": "1234-05-06T07:08:09Z" } },
                { "c": { "$lt": "1234-05-06T07:08:09Z" } }
            ] }
        ] }"#)
        .unwrap();
        let mut filter = JsonObject::new();
        extend_filter(&mut filter, clause, None);
        assert_eq!(
            filter,
            json_object!({
                MINIMUM_SHOULD_MATCH: 1,
                SHOULD: [
                    { BOOL: { FILTER: [term, range] } },
                    { BOOL: { MINIMUM_SHOULD_MATCH: 1, SHOULD: [term, range_gt, range_lt] } },
                    { BOOL: { MINIMUM_SHOULD_MATCH: 1, SHOULD: [term, range_gt, range_lt] } }
                ]
            }),
        );
    }
}
