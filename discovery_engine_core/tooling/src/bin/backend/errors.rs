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

#[derive(Error, Debug, DisplayDoc)]
pub(crate) enum BackendError {
    /// Elastic search error: {0}
    Elastic(#[source] reqwest::Error),
    /// Error receiving response: {0}
    Receiving(#[source] reqwest::Error),
    /// Error searching news with no history
    NoHistory,
}

impl actix_web::error::ResponseError for BackendError {}
