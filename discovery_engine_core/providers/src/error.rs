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

use displaydoc::Display as DisplayDoc;
use thiserror::Error;

/// Client errors.
#[derive(Error, Debug, DisplayDoc)]
pub enum Error {
    /// Invalid API Url base
    InvalidUrlBase(Option<url::ParseError>),
    /// Failed to execute the HTTP request: {0}
    RequestExecution(#[source] reqwest::Error),
    /// Server returned a non-successful status code: {0}
    StatusCode(#[source] reqwest::Error),
    /// Failed to fetch from the server: {0}
    Fetching(#[source] reqwest::Error),
    /// Failed to decode the server's response: {0}
    Decoding(#[source] serde_json::Error),
    /// Failed to encode the query: {0}
    Encoding(#[source] serde_json::Error),
    /// Failed to decode the server's response at JSON path {1}: {0}
    DecodingAtPath(
        String,
        #[source] serde_path_to_error::Error<serde_json::Error>,
    ),
    /// Impossible to parse the provided url: {0}.
    InvalidUrl(#[from] url::ParseError),
    /// The provided Url is missing a domain: {0}
    MissingDomainInUrl(String),
    /// None of the received articles were well-formed. See trace logs for details.
    NoValidArticles,
    /// In the configuration a URL is malformed/unsupported: {0}
    MalformedUrlInConfig(#[source] url::ParseError),
    /// In the configuration a URL path is malformed/unsupported: {path}
    MalformedUrlPathInConfig { path: String },
    /// We can't detect which provider to use for given endpoint: {url}
    NoProviderForEndpoint { url: String },
}
