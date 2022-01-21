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
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Query definition.
use std::collections::HashMap;

use crate::filter::Filter;

/// Query to do to the provider.
pub(crate) struct Query<'filter> {
    /// API token.
    token: String,

    /// Number of results to return. Between 1 and 100, default 100.
    size: u8,

    /// Filter that define which posts will be returned.
    filter: &'filter Filter,
}

impl<'filter> Query<'filter> {
    pub(crate) fn new(token: String, filter: &'filter Filter, size: Option<u8>) -> Self {
        Self {
            token,
            filter,
            size: size.unwrap_or(100),
        }
    }
}

impl<'filter, S> From<Query<'filter>> for HashMap<String, String, S>
where
    S: std::hash::BuildHasher + Default,
{
    fn from(query: Query<'filter>) -> HashMap<String, String, S> {
        std::array::IntoIter::new([
            ("token", query.token),
            ("size", query.size.to_string()),
            ("q", query.filter.build()),
            ("sort", "relevancy".to_string()),
            ("format", "json".to_string()),
        ])
        .map(|(k, v)| (k.to_string(), v))
        .collect()
    }
}
