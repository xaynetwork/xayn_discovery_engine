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

//! Client for "more like this" queries.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use url::Url;

use crate::{
    newscatcher::{append_market, max_age_to_date_string, to_generic_articles},
    Error,
    GenericArticle,
    NewscatcherResponse,
    RestEndpoint,
    SimilarNewsProvider,
    SimilarNewsQuery,
};

pub(crate) struct MltSimilarNewsProvider {
    endpoint: RestEndpoint,
}

#[async_trait]
impl SimilarNewsProvider for MltSimilarNewsProvider {
    async fn query_similar_news(
        &self,
        query: &SimilarNewsQuery<'_>,
    ) -> Result<Vec<GenericArticle>, Error> {
        let response = self
            .endpoint
            .get_request::<NewscatcherResponse, _>(|query_append| {
                query_append("like", query.like.to_string());
                query_append("min_term_freq", "1".to_string());

                query_append("page_size", query.page_size.to_string());
                query_append("page", query.page.to_string());

                if !query.excluded_sources.is_empty() {
                    query_append("not_sources", query.excluded_sources.join(","));
                }

                query_append("sort_by", "relevancy".to_owned());
                append_market(query_append, query.market, &query.rank_limit);

                if let Some(days) = &query.max_age_days {
                    query_append("from", max_age_to_date_string(*days));
                }
            })
            .await?;

        to_generic_articles(response.articles)
    }
}

impl MltSimilarNewsProvider {
    #[allow(dead_code)] // TEMP
    pub(crate) fn new(endpoint_url: Url, auth_token: String, timeout: Duration) -> Self {
        Self {
            endpoint: RestEndpoint::new(endpoint_url, auth_token, timeout),
        }
    }

    pub(crate) fn from_endpoint(endpoint: RestEndpoint) -> Arc<dyn SimilarNewsProvider> {
        Arc::new(Self { endpoint })
    }
}
