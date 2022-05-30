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

//! Client to get new documents.

use crate::{
    gnews::{Client as GnewsClient, NewsQuery as GnewsNewsQuery},
    newscatcher::Client as NewscatcherClient,
    Article,
    Error,
    GnewsHeadlinesQuery,
    NewscatcherQuery,
};

/// Client that can provide documents.
pub struct Client {
    newscatcher: NewscatcherClient,
    gnews: GnewsClient,
}

impl Client {
    /// Create a client.
    pub fn new(token: impl Into<String>, url: impl Into<String>) -> Self {
        let token: String = token.into();
        let url: String = url.into();
        Self {
            newscatcher: NewscatcherClient::new(token.clone(), url.clone()),
            gnews: GnewsClient::new(token, url),
        }
    }

    /// Run a query for fetching `Article`s.
    pub async fn query_gnews_articles(
        &self,
        query: &GnewsNewsQuery<'_>,
    ) -> Result<Vec<Article>, Error> {
        self.gnews.query_articles(query).await
    }

    /// Run a query for fetching `Article`s.
    pub async fn query_gnews_headlines(
        &self,
        query: &GnewsHeadlinesQuery<'_>,
    ) -> Result<Vec<Article>, Error> {
        self.gnews.query_headlines(query).await
    }

    /// Run a query for fetching `Article`s from the newscatcher API.
    pub async fn query_newscatcher(
        &self,
        query: &impl NewscatcherQuery,
    ) -> Result<Vec<Article>, Error> {
        self.newscatcher.query_articles(query).await
    }
}
