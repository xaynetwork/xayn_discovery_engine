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

use serde_json::{json, Value};
use xayn_web_api_shared::serde::JsonObject;

use crate::{
    personalization::filter::{CompareOp, Filter},
    storage::KnnSearchParams,
};

fn extend_filter(filter: &mut JsonObject, clause: &Filter) {
    let clause = match clause {
        Filter::Compare(compare) => {
            let compare_with = json!({ format!("properties.{}", compare.field): compare.value });
            let compare = match compare.operation {
                CompareOp::Eq => "term",
                CompareOp::In => "terms",
            };
            json!({ compare: compare_with })
        }
    };
    if let Some(filter) = filter.get_mut("filter") {
        match filter {
            Value::Object(_) => {
                *filter = json!([filter.take(), clause]);
            }
            Value::Array(filter) => filter.push(clause),
            _ => unreachable!(/* filter is object or array */),
        }
    } else {
        filter.insert("filter".to_string(), clause);
    }
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
            extend_filter(&mut filter, opt_filter);
        }

        filter
    }
}

#[cfg(test)]
mod tests {
    use xayn_web_api_shared::serde::json_object;

    use super::*;

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
            extend_filter(&mut filter, clause);
            assert_eq!(filter, json_object!({ "filter": term }));

            let mut filter = json_object!({ "filter": {} });
            extend_filter(&mut filter, clause);
            assert_eq!(filter, json_object!({ "filter": [{}, term] }));

            let mut filter = json_object!({ "filter": [{}, {}] });
            extend_filter(&mut filter, clause);
            assert_eq!(filter, json_object!({ "filter": [{}, {}, term] }));
        }
    }
}
