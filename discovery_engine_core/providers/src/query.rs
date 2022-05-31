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

#![allow(clippy::module_name_repetitions)]

//! Module containing query structs usable across all providers.
//!
//! If a provider doesn't support a field in a query they should ignore it.

use chrono::Utc;

use crate::{Filter, Market};

/// Elements shared between various headlines and news queries.
///
/// Fields not supported by the used provider will be ignored.
pub struct CommonQueryParts<'a> {
    /// How many articles to return (per page).
    pub page_size: usize,

    /// The number of the page which should be returned.
    ///
    /// Paging starts with `1`.
    pub page: usize,

    /// Exclude given sources.
    pub excluded_sources: &'a [String],
}

/// Parameters determining which news to fetch
///
/// Fields not supported by the used provider will be ignored.
pub struct NewsQuery<'a> {
    /// Common parts
    pub common: CommonQueryParts<'a>,

    /// Market of news.
    pub market: &'a Market,

    /// News search filter.
    pub filter: &'a Filter,

    //FIXME gnews support for from
    /// Starting point in time from which to start the search.
    /// The format is YYYY/mm/dd. Default timezone is UTC.
    /// Defaults to the last week.
    pub from: Option<String>,
}

/// Parameters determining which headlines to fetch.
///
/// Fields not supported by the used provider will be ignored.
pub struct HeadlinesQuery<'a> {
    /// Common parts.
    pub common: CommonQueryParts<'a>,

    /// Market of news.
    pub market: &'a Market,

    /// Headlines topic.
    pub topic: Option<&'a str>,

    //FIXME gnews support for from derived from when
    /// The time period you want to get the latest headlines for.
    /// Can be specified in days (e.g. 3d) or hours (e.g. 24h).
    /// Defaults to all data available for the subscriptions.
    pub when: Option<&'a str>,
}

/// Parameters determining which which headlines from trusted sources to fetch.
///
/// Fields not supported by the used provider will be ignored.
pub struct TrustedSourcesQuery<'a> {
    /// Common parts.
    pub common: CommonQueryParts<'a>,

    /// Prefer trusted sources
    pub trusted_sources: &'a [String],

    //FIXME gnews support for from derived from when
    /// The time period you want to get the latest headlines for.
    /// Can be specified in days (e.g. 3d) or hours (e.g. 24h).
    /// Defaults to all data available for the subscriptions.
    pub when: Option<&'a str>,
}

//FIXME more clear name
/// Default `from` value for newscatcher news queries
pub fn default_from() -> String {
    let from = Utc::today() - chrono::Duration::days(3);
    from.format("%Y/%m/%d").to_string()
}

//FIXME more clear name
/// Default `when` value for newscatcher headline queries
pub const DEFAULT_WHEN: Option<&'static str> = Some("3d");
