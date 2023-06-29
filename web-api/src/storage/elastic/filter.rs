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
use xayn_web_api_shared::serde::JsonObject;

use crate::{
    personalization::filter::{CombineOp, CompareOp, Filter},
    storage::KnnSearchParams,
};

fn extend_value(filter: &mut JsonObject, occurence: &'static str, mut clause: Value) {
    if let Some(filter) = filter.get_mut(occurence) {
        match (&mut *filter, &mut clause) {
            (Value::Array(filter), Value::Array(clause)) => filter.append(clause),
            (Value::Array(filter), Value::Object(_)) => filter.push(clause),
            (Value::Object(_), Value::Array(clauses)) => {
                clauses.insert(0, filter.take());
                *filter = clause;
            }
            (Value::Object(_), Value::Object(_)) => {
                *filter = json!([filter.take(), clause]);
            }
            _ => unreachable!(/* filter and clause are array or object */),
        }
    } else {
        filter.insert(occurence.to_string(), clause);
    }
}

fn extend_filter(filter: &mut JsonObject, clause: &Filter, is_not_root: Option<&'static str>) {
    const FILTER: &str = "filter";
    const SHOULD: &str = "should";
    const MINIMUM_SHOULD_MATCH: &str = "minimum_should_match";

    match clause {
        Filter::Compare(compare) => {
            let compare_with = json!({ format!("properties.{}", compare.field): compare.value });
            let compare = match compare.operation {
                CompareOp::Eq => "term",
                CompareOp::In => "terms",
            };
            extend_value(
                filter,
                is_not_root.unwrap_or(FILTER),
                json!({ compare: compare_with }),
            );
        }
        Filter::Combine(combine) => {
            let (occurence, clauses) = match (combine.operation, is_not_root) {
                (CombineOp::And, _) => (FILTER, JsonObject::with_capacity(combine.filters.len())),
                (CombineOp::Or, Some(_)) => {
                    let mut clauses = JsonObject::with_capacity(combine.filters.len() + 1);
                    clauses.insert(MINIMUM_SHOULD_MATCH.to_string(), json!(1));
                    (SHOULD, clauses)
                }
                (CombineOp::Or, None) => {
                    filter.insert(MINIMUM_SHOULD_MATCH.to_string(), json!(1));
                    (SHOULD, JsonObject::with_capacity(combine.filters.len()))
                }
            };
            let clauses = combine.filters.iter().fold(clauses, |mut filter, clause| {
                extend_filter(&mut filter, clause, Some(occurence));
                filter
            });
            if is_not_root.is_some() {
                extend_value(filter, occurence, json!({ "bool": clauses }));
            } else {
                let mut clauses = clauses.into_iter().map(|(_, clause)| clause).collect_vec();
                match clauses.len() {
                    0 => {}
                    1 => extend_value(filter, occurence, clauses.remove(0)),
                    _ => extend_value(filter, occurence, json!(clauses)),
                }
            }
        }
    };
}

impl KnnSearchParams<'_> {
    pub(super) fn create_search_filter(&self) -> JsonObject {
        let mut filter = JsonObject::new();
        if !self.excluded.is_empty() {
            // existing documents are not filtered in the query to avoid too much work for a cold
            // path, filtering them afterwards can occasionally lead to less than k results though
            filter.insert(
                "must_not".to_string(),
                json!({ "ids": { "values": self.excluded } }),
            );
        }
        if let Some(published_after) = self.published_after {
            // published_after != null && published_after <= publication_date
            let published_after = published_after.to_rfc3339();
            filter.insert(
                "filter".to_string(),
                json!({ "range": { "properties.publication_date": { "gte": published_after } } }),
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

    #[test]
    fn test_extend_value() {
        let mut filter = JsonObject::new();
        extend_value(&mut filter, "test", json!([{}, {}]));
        assert_eq!(filter, json_object!({ "test": [{}, {}] }));
        extend_value(&mut filter, "test", json!([{}]));
        assert_eq!(filter, json_object!({ "test": [{}, {}, {}] }));

        let mut filter = json_object!({ "test": [{}] });
        extend_value(&mut filter, "test", json!([{}, {}]));
        assert_eq!(filter, json_object!({ "test": [{}, {}, {}] }));

        let mut filter = JsonObject::new();
        extend_value(&mut filter, "test", json!({}));
        assert_eq!(filter, json_object!({ "test": {} }));
        extend_value(&mut filter, "test", json!({}));
        assert_eq!(filter, json_object!({ "test": [{}, {}] }));

        let mut filter = json_object!({ "test": [{}] });
        extend_value(&mut filter, "test", json!({}));
        assert_eq!(filter, json_object!({ "test": [{}, {}] }));
    }

    #[test]
    fn test_extend_filter_compare() {
        for (clause, term) in [
            (
                &serde_json::from_str(r#"{ "a": { "$eq": "b" } }"#).unwrap(),
                json_object!({ "term": { "properties.a": "b" } }),
            ),
            (
                &serde_json::from_str(r#"{ "a": { "$in": ["b", "c"] } }"#).unwrap(),
                json_object!({ "terms": { "properties.a": ["b", "c"] } }),
            ),
        ] {
            let mut filter = JsonObject::new();
            extend_filter(&mut filter, clause, None);
            assert_eq!(filter, json_object!({ "filter": term }));

            let mut filter = json_object!({ "filter": {} });
            extend_filter(&mut filter, clause, None);
            assert_eq!(filter, json_object!({ "filter": [{}, term] }));

            let mut filter = json_object!({ "filter": [{}, {}] });
            extend_filter(&mut filter, clause, None);
            assert_eq!(filter, json_object!({ "filter": [{}, {}, term] }));
        }
    }

    #[test]
    fn test_extend_filter_combine_and() {
        let clause = &serde_json::from_str(
            r#"{ "$and": [{ "a": { "$eq": "b" } }, { "a": { "$eq": "b" } }] }"#,
        )
        .unwrap();
        let term = json_object!({ "term": { "properties.a": "b" } });

        let mut filter = JsonObject::new();
        extend_filter(&mut filter, clause, None);
        assert_eq!(filter, json_object!({ "filter": [term, term] }));

        let mut filter = json_object!({ "filter": {} });
        extend_filter(&mut filter, clause, None);
        assert_eq!(filter, json_object!({ "filter": [{}, term, term] }));

        let mut filter = json_object!({ "filter": [{}, {}] });
        extend_filter(&mut filter, clause, None);
        assert_eq!(filter, json_object!({ "filter": [{}, {}, term, term] }));
    }

    #[test]
    fn test_extend_filter_combine_or() {
        let clause = &serde_json::from_str(
            r#"{ "$or": [{ "a": { "$eq": "b" } }, { "a": { "$eq": "b" } }] }"#,
        )
        .unwrap();
        let term = json_object!({ "term": { "properties.a": "b" } });

        let mut filter = JsonObject::new();
        extend_filter(&mut filter, clause, None);
        assert_eq!(
            filter,
            json_object!({ "should": [term, term], "minimum_should_match": 1 }),
        );

        let mut filter = json_object!({ "filter": {} });
        extend_filter(&mut filter, clause, None);
        assert_eq!(
            filter,
            json_object!({ "filter": {}, "should": [term, term], "minimum_should_match": 1 }),
        );
    }

    #[test]
    fn test_extend_filter_nested() {
        let clause = &serde_json::from_str(
            r#"{ "$and": [
                { "$and": [{ "a": { "$eq": "b" } }, { "a": { "$eq": "b" } }] },
                { "$or": [{ "a": { "$eq": "b" } }, { "a": { "$eq": "b" } }] }
            ] }"#,
        )
        .unwrap();
        let term = json_object!({ "term": { "properties.a": "b" } });

        let mut filter = JsonObject::new();
        extend_filter(&mut filter, clause, None);
        assert_eq!(
            filter,
            json_object!({ "filter": [
                { "bool": { "filter": [term, term] } },
                { "bool": { "should": [term, term], "minimum_should_match": 1 } }
            ] }),
        );

        let clause = &serde_json::from_str(
            r#"{ "$or": [
                { "$and": [{ "a": { "$eq": "b" } }, { "a": { "$eq": "b" } }] },
                { "$or": [{ "a": { "$eq": "b" } }, { "a": { "$eq": "b" } }] }
            ] }"#,
        )
        .unwrap();
        let mut filter = JsonObject::new();
        extend_filter(&mut filter, clause, None);
        assert_eq!(
            filter,
            json_object!({
                "should": [
                    { "bool": { "filter": [term, term] } },
                    { "bool": { "should": [term, term], "minimum_should_match": 1 } }
                ],
                "minimum_should_match": 1
            }),
        );
    }
}
