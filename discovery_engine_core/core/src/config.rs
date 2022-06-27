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

use std::sync::Arc;

use figment::{
    providers::{Format, Json, Serialized},
    Figment,
};
use tokio::sync::RwLock;

use xayn_discovery_engine_ai::CoiSystemConfig;
use xayn_discovery_engine_providers::Market;

/// Configuration settings to initialize Discovery Engine with a
/// [`xayn_discovery_engine_ai::Ranker`].
#[derive(Clone)]
pub struct InitConfig {
    /// Key for accessing the API.
    pub api_key: String,
    /// API base url.
    pub api_base_url: String,
    /// List of markets to use.
    pub markets: Vec<Market>,
    /// List of trusted sources to use.
    pub trusted_sources: Vec<String>,
    /// List of excluded sources to use.
    pub excluded_sources: Vec<String>,
    /// S-mBert vocabulary path.
    pub smbert_vocab: String,
    /// S-mBert model path.
    pub smbert_model: String,
    /// KPE vocabulary path.
    pub kpe_vocab: String,
    /// KPE model path.
    pub kpe_model: String,
    /// KPE CNN path.
    pub kpe_cnn: String,
    /// KPE classifier path.
    pub kpe_classifier: String,
    /// AI config in JSON format.
    pub ai_config: Option<String>,
}

/// Discovery Engine endpoint settings.
pub(crate) struct EndpointConfig {
    /// Page size setting for API.
    pub(crate) page_size: usize,
    /// Write-exclusive access to markets list.
    pub(crate) markets: Arc<RwLock<Vec<Market>>>,
    /// Trusted sources for news queries.
    pub(crate) trusted_sources: Arc<RwLock<Vec<String>>>,
    /// Sources to exclude for news queries.
    pub(crate) excluded_sources: Arc<RwLock<Vec<String>>>,
    /// The maximum number of requests to try to reach the number of `min_articles`.
    pub(crate) max_requests: u32,
    /// The minimum number of new articles to try to return when updating the stack.
    pub(crate) min_articles: usize,
    /// The maximum age of a headline, in days, after which we no longer
    /// want to display them
    pub(crate) max_headline_age_days: usize,
    /// The maximum age of a news article, in days, after which we no longer
    /// want to display them
    pub(crate) max_article_age_days: usize,
}

impl From<InitConfig> for EndpointConfig {
    fn from(config: InitConfig) -> Self {
        Self {
            page_size: 100,
            markets: Arc::new(RwLock::new(config.markets)),
            trusted_sources: Arc::new(RwLock::new(config.trusted_sources)),
            excluded_sources: Arc::new(RwLock::new(config.excluded_sources)),
            max_requests: 5,
            min_articles: 20,
            max_headline_age_days: 3,
            max_article_age_days: 30,
        }
    }
}

/// Internal config to allow for configurations within the core without a mirroring outside impl.
pub(crate) struct CoreConfig {
    /// The number of taken top key phrases while updating the stacks.
    pub(crate) take_top: usize,
    /// The number of top documents per stack to keep while filtering the stacks.
    pub(crate) keep_top: usize,
    /// The lower bound of documents per stack at which new items are requested.
    pub(crate) request_new: usize,
    /// The number of times to get feed documents after which the stacks are updated without the
    /// limitation of `request_new`.
    pub(crate) request_after: usize,
    /// The maximum number of top key phrases extracted from the search term in the deep search.
    pub(crate) deep_search_top: usize,
    /// The maximum number of documents returned from the deep search.
    pub(crate) deep_search_max: usize,
    /// The minimum cosine similarity wrt the original document below which documents returned from
    /// the deep search are discarded.
    pub(crate) deep_search_sim: f32,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            take_top: 3,
            keep_top: 20,
            request_new: 3,
            request_after: 2,
            deep_search_top: 3,
            deep_search_max: 20,
            deep_search_sim: 0.2,
        }
    }
}

/// Reads the configurations from json and sets defaults for missing fields.
pub(crate) fn config_from_json(json: &str) -> Figment {
    Figment::new()
        .merge(Serialized::defaults(CoiSystemConfig::default()))
        .merge(Serialized::default("kpe.token_size", 150))
        .merge(Serialized::default("smbert.token_size", 150))
        .merge(Json::string(json))
}

#[cfg(test)]
mod tests {
    use xayn_discovery_engine_ai::GenericError;

    use super::*;

    #[test]
    fn test_config_from_json_default() -> Result<(), GenericError> {
        let ai_config = config_from_json("{}");
        assert_eq!(ai_config.extract_inner::<usize>("kpe.token_size")?, 150);
        assert_eq!(ai_config.extract_inner::<usize>("smbert.token_size")?, 150);
        assert_eq!(
            ai_config.extract::<CoiSystemConfig>()?,
            CoiSystemConfig::default(),
        );
        Ok(())
    }

    #[test]
    fn test_config_from_json_modified() -> Result<(), GenericError> {
        let ai_config = config_from_json(
            r#"{
                "coi": {
                    "threshold": 0.42
                },
                "kpe": {
                    "penalty": [0.99, 0.66, 0.33]
                },
                "smbert": {
                    "token_size": 42,
                    "foo": "bar"
                },
                "baz": 0
            }"#,
        );
        assert_eq!(ai_config.extract_inner::<usize>("kpe.token_size")?, 150);
        assert_eq!(ai_config.extract_inner::<usize>("smbert.token_size")?, 42);
        assert_eq!(
            ai_config.extract::<CoiSystemConfig>()?,
            CoiSystemConfig::default()
                .with_threshold(0.42)?
                .with_penalty(&[0.99, 0.66, 0.33])?,
        );
        Ok(())
    }
}
