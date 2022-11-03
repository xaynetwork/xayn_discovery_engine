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

use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    error::Error,
    models::{content::GenericArticle, query::SimilarSearchQuery},
    newscatcher::{append_market, max_age_to_date_string, to_generic_articles, Response},
    utils::rest_endpoint::RestEndpoint,
    SimilarSearchProvider,
};

pub struct MltSimilarSearchProvider {
    endpoint: RestEndpoint,
}

impl MltSimilarSearchProvider {
    pub fn from_endpoint(endpoint: RestEndpoint) -> Arc<dyn SimilarSearchProvider> {
        Arc::new(Self { endpoint })
    }
}

#[async_trait]
impl SimilarSearchProvider for MltSimilarSearchProvider {
    async fn query_similar_search(
        &self,
        query: &SimilarSearchQuery<'_>,
    ) -> Result<Vec<GenericArticle>, Error> {
        let response = self
            .endpoint
            .get_request::<_, Response>(|query_append| {
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
