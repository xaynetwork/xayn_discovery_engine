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

pub(crate) mod filter;
mod knn;
mod rerank;
pub(crate) mod routes;
pub(crate) mod shared;
mod stateless;

use std::ops::RangeBounds;

use anyhow::bail;
use serde::{Deserialize, Serialize};

pub use self::{rerank::bench_rerank, stateless::bench_derive_interests};
use crate::app::SetupError;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub(crate) struct PersonalizationConfig {
    /// Max number of documents to return.
    pub(crate) max_number_documents: usize,

    /// Max number of document candidates for aKNN. The number of requested documents (`count`) must
    /// by <= this, which can be guaranteed by setting this >= `max_number_documents`.
    pub(crate) max_number_candidates: usize,

    /// Default number of documents to return.
    pub(crate) default_number_documents: usize,

    /// Max number of cois to use in knn search.
    pub(crate) max_cois_for_knn: usize,

    /// Weights for reranking of the scores. Each weight is in `[0, 1]` and they add up to `1`. The
    /// order is `[interest_weight, tag_weight, elasticsearch_weight]`.
    pub(crate) score_weights: [f32; 3],

    /// Whether to store the history of user interactions.
    pub(crate) store_user_history: bool,

    /// The maximal number of history entries used as stateless user history.
    pub(crate) max_stateless_history_size: usize,

    /// The maximal number of history entries used when calculating CoIs from a stateless user history.
    pub(crate) max_stateless_history_for_cois: usize,
}

impl Default for PersonalizationConfig {
    fn default() -> Self {
        Self {
            max_number_documents: 100,
            max_number_candidates: 100,
            default_number_documents: 10,
            // FIXME: what is a default value we know works well with how we do knn?
            max_cois_for_knn: 10,
            score_weights: [1., 1., 0.],
            store_user_history: true,
            max_stateless_history_size: 200,
            max_stateless_history_for_cois: 20,
        }
    }
}

impl PersonalizationConfig {
    pub(crate) fn validate(&self) -> Result<(), SetupError> {
        if self.max_number_documents > self.max_number_candidates {
            // this is stricter than necessary, but ok for our use cases
            bail!("invalid PersonalizationConfig, max_number_documents must be <= max_number_candidates");
        }
        if self.default_number_documents > self.max_number_documents {
            bail!("invalid PersonalizationConfig, default_number_documents must be <= max_number_documents");
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub(crate) struct SemanticSearchConfig {
    /// Max number of documents to return.
    pub(crate) max_number_documents: usize,

    /// Max number of document candidates for aKNN. The number of requested documents (`count`) must
    /// by <= this, which can be guaranteed by setting this >= `max_number_documents`.
    pub(crate) max_number_candidates: usize,

    /// Default number of documents to return.
    pub(crate) default_number_documents: usize,

    /// Weights for reranking of the scores. Each weight is in `[0, 1]` and they add up to `1`. The
    /// order is `[interest_weight, tag_weight, elasticsearch_weight]`.
    pub(crate) score_weights: [f32; 3],

    /// Max number of bytes a query can have
    ///
    /// Hint: Use [`Self.query_size_bounds()`] to access this.
    max_query_size: usize,
}

impl SemanticSearchConfig {
    pub(crate) fn query_size_bounds(&self) -> impl RangeBounds<usize> + Clone {
        1..=self.max_query_size
    }
}

impl Default for SemanticSearchConfig {
    fn default() -> Self {
        Self {
            max_number_documents: 100,
            max_number_candidates: 100,
            default_number_documents: 10,
            score_weights: [1., 1., 0.5],
            max_query_size: 512,
        }
    }
}

impl SemanticSearchConfig {
    pub(crate) fn validate(&self) -> Result<(), SetupError> {
        if self.max_number_documents > self.max_number_candidates {
            // this is stricter than necessary, but ok for our use cases
            bail!("invalid SemanticSearchConfig, max_number_documents must be <= max_number_candidates");
        }
        if self.default_number_documents > self.max_number_documents {
            bail!("invalid SemanticSearchConfig, default_number_documents must be <= max_number_documents");
        }
        if self.max_query_size < 1 {
            bail!("max_query_size needs to be at least 1");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_default_personalization_config() {
        PersonalizationConfig::default().validate().unwrap();
    }

    #[test]
    fn test_validate_default_semantic_search_config() {
        PersonalizationConfig::default().validate().unwrap();
    }
}
