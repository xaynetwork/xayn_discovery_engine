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

const FILTER: &str = "filter";
const MINIMUM_SHOULD_MATCH: &str = "minimum_should_match";
const SHOULD: &str = "should";
const TERM: &str = "term";
const TERMS: &str = "terms";

fn extend_value(filter: &mut JsonObject, occurence: &'static str, clause: Value) {
    if let Some(filter) = filter.get_mut(occurence) {
        let (Value::Array(filter), Value::Array(clause)) = (&mut *filter, clause) else {
            unreachable!(/* filter[occurence] and clause are arrays */);
        };
        filter.extend(clause);
    } else {
        filter.insert(occurence.to_string(), clause);
    }
}

fn extend_filter(filter: &mut JsonObject, clause: &Filter, is_not_root: Option<&'static str>) {
    match clause {
        Filter::Compare(compare) => {
            let clause = json!({ format!("properties.{}", compare.field): compare.value });
            let clause = match compare.operation {
                CompareOp::Eq => json!([{ TERM: clause }]),
                CompareOp::In => json!([{ TERMS: clause }]),
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
                json!([{ "bool": clause }])
            } else {
                json!(clause
                    .into_iter()
                    .flat_map(|(_, clause)| {
                        let Value::Array(clause) = clause else {
                            unreachable!(/* clause is array */);
                        };
                        clause
                    })
                    .collect_vec())
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
            // existing documents are not filtered in the query to avoid too much work for a cold
            // path, filtering them afterwards can occasionally lead to less than k results though
            filter.insert(
                "must_not".to_string(),
                json!([{ "ids": { "values": self.excluded } }]),
            );
        }
        if let Some(published_after) = self.published_after {
            // published_after != null && published_after <= publication_date
            let published_after = published_after.to_rfc3339();
            filter.insert(
                FILTER.to_string(),
                json!([{ "range": { "properties.publication_date": { "gte": published_after } } }]),
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
        extend_value(&mut filter, FILTER, json!([]));
        assert_eq!(filter, json_object!({ FILTER: [] }));
        extend_value(&mut filter, FILTER, json!([{}]));
        assert_eq!(filter, json_object!({ FILTER: [{}] }));

        let mut filter = JsonObject::new();
        extend_value(&mut filter, FILTER, json!([{}]));
        assert_eq!(filter, json_object!({ FILTER: [{}] }));
        extend_value(&mut filter, FILTER, json!([{}, {}]));
        assert_eq!(filter, json_object!({ FILTER: [{}, {}, {}] }));

        let mut filter = json_object!({ SHOULD: [] });
        extend_value(&mut filter, FILTER, json!([{}]));
        assert_eq!(filter, json_object!({ SHOULD: [], FILTER: [{}] }));
    }

    #[test]
    fn test_extend_filter_compare() {
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
    fn test_extend_filter_combine_and() {
        let clause = &serde_json::from_str(
            r#"{ "$and": [{ "a": { "$eq": "b" } }, { "a": { "$eq": "b" } }] }"#,
        )
        .unwrap();
        let term = json!({ TERM: { "properties.a": "b" } });

        let mut filter = JsonObject::new();
        extend_filter(&mut filter, clause, None);
        assert_eq!(filter, json_object!({ FILTER: [term, term] }));

        let mut filter = json_object!({ FILTER: [] });
        extend_filter(&mut filter, clause, None);
        assert_eq!(filter, json_object!({ FILTER: [term, term] }));

        let mut filter = json_object!({ FILTER: [{}] });
        extend_filter(&mut filter, clause, None);
        assert_eq!(filter, json_object!({ FILTER: [{}, term, term] }));

        let mut filter = json_object!({ SHOULD: [] });
        extend_filter(&mut filter, clause, None);
        assert_eq!(filter, json_object!({ SHOULD: [], FILTER: [term, term] }),);
    }

    #[test]
    fn test_extend_filter_combine_or() {
        let clause = &serde_json::from_str(
            r#"{ "$or": [{ "a": { "$eq": "b" } }, { "a": { "$eq": "b" } }] }"#,
        )
        .unwrap();
        let term = json_object!({ TERM: { "properties.a": "b" } });

        let mut filter = JsonObject::new();
        extend_filter(&mut filter, clause, None);
        assert_eq!(
            filter,
            json_object!({ SHOULD: [term, term], MINIMUM_SHOULD_MATCH: 1 }),
        );

        let mut filter = json_object!({ SHOULD: [] });
        extend_filter(&mut filter, clause, None);
        assert_eq!(
            filter,
            json_object!({ SHOULD: [term, term], MINIMUM_SHOULD_MATCH: 1 }),
        );

        let mut filter = json_object!({ SHOULD: [{}] });
        extend_filter(&mut filter, clause, None);
        assert_eq!(
            filter,
            json_object!({ SHOULD: [{}, term, term], MINIMUM_SHOULD_MATCH: 1 }),
        );

        let mut filter = json_object!({ FILTER: [] });
        extend_filter(&mut filter, clause, None);
        assert_eq!(
            filter,
            json_object!({ FILTER: [], SHOULD: [term, term], MINIMUM_SHOULD_MATCH: 1 }),
        );
    }

    #[test]
    fn test_extend_filter_nested() {
        let clause = &serde_json::from_str(
            r#"{ "$and": [
                { "$and": [{ "a": { "$eq": "b" } }, { "a": { "$eq": "b" } }] },
                { "$or": [{ "a": { "$eq": "b" } }, { "a": { "$eq": "b" } }] },
                { "$and": [{ "a": { "$eq": "b" } }, { "a": { "$eq": "b" } }] }
            ] }"#,
        )
        .unwrap();
        let term = json_object!({ TERM: { "properties.a": "b" } });

        let mut filter = JsonObject::new();
        extend_filter(&mut filter, clause, None);
        assert_eq!(
            filter,
            json_object!({ FILTER: [
                { "bool": { FILTER: [term, term] } },
                { "bool": { FILTER: [term, term] } },
                { "bool": { SHOULD: [term, term], MINIMUM_SHOULD_MATCH: 1 } }
            ] }),
        );

        let clause = &serde_json::from_str(
            r#"{ "$or": [
                { "$or": [{ "a": { "$eq": "b" } }, { "a": { "$eq": "b" } }] },
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
                MINIMUM_SHOULD_MATCH: 1,
                SHOULD: [
                    { "bool": { FILTER: [term, term] } },
                    { "bool": { MINIMUM_SHOULD_MATCH: 1, SHOULD: [term, term] } },
                    { "bool": { MINIMUM_SHOULD_MATCH: 1, SHOULD: [term, term] } }
                ]
            }),
        );
    }
}
