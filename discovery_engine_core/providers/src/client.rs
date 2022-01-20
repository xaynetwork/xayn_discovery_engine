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

//! Client do get new documents.

use xayn_discovery_engine_core::document::Document;

use crate::{filter::Filter, query::Query};

// TODO: replace with error form the http library + Filter error
pub enum Error {}

/// Client that can provide documents.
pub struct Client {
    token: String,
    url: String,
}

impl Client {
    /// Create a client.
    pub fn new(token: String, url: String) -> Self {
        Self { token, url }
    }

    /// Get document from a provider,
    ///
    /// `filter` can be used to query only documents of interests,
    /// `size` optionally limit the number of item we want.
    pub async fn get(&self, filter: Filter, size: Option<u8>) -> Result<Vec<Document>, Error> {
        let _query = Query::new(self.token.clone(), filter, size);

        // just to use it until we really use it
        let _url = self.url.clone();

        Ok(vec![])
    }
}
