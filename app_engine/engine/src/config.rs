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

use std::{collections::HashSet, sync::Arc};

use figment::{
    providers::{Format, Json, Serialized},
    Figment,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use xayn_ai_coi::CoiConfig;
use xayn_discovery_engine_ai::KpsConfig;
use xayn_discovery_engine_providers::Market;

use crate::{
    engine::Engine,
    stack::{
        exploration::Stack as Exploration,
        ops::{breaking::BreakingNews, personalized::PersonalizedNews, trusted::TrustedNews},
    },
    storage::DartMigrationData,
};

impl Engine {
    pub fn endpoint_config(&self) -> &EndpointConfig {
        &self.endpoint_config
    }

    pub fn core_config(&self) -> &CoreConfig {
        &self.core_config
    }

    pub fn feed_config(&self) -> &FeedConfig {
        &self.feed_config
    }

    pub fn search_config(&self) -> &SearchConfig {
        &self.search_config
    }

    pub fn coi_config(&self) -> &CoiConfig {
        self.coi.config()
    }

    pub fn kps_config(&self) -> &KpsConfig {
        self.kps.config()
    }

    pub fn exploration_config(&self) -> &ExplorationConfig {
        &self.exploration_stack.config
    }
}

/// Configuration settings to initialize the Discovery [`Engine`].
///
/// [`Engine`]: crate::engine::Engine
#[derive(Clone, Debug)]
pub struct InitConfig {
    /// Key for accessing the API.
    pub api_key: String,
    /// API base url.
    pub api_base_url: String,
    /// Route for the news provider.
    pub news_provider: Option<String>,
    /// Route for the similar news provider.
    pub similar_news_provider: Option<String>,
    /// Route for the headlines provider.
    pub headlines_provider: Option<String>,
    /// Route for the trusted headlines provider.
    pub trusted_headlines_provider: Option<String>,
    /// List of markets to use.
    pub markets: Vec<Market>,
    /// Bert path.
    pub bert: String,
    /// The maximum number of documents per feed batch.
    pub max_docs_per_feed_batch: u32,
    /// The maximum number of documents per search batch.
    pub max_docs_per_search_batch: u32,
    /// DE config in JSON format.
    pub de_config: Option<String>,
    /// Log file path.
    pub log_file: Option<String>,
    /// Directory in which user data should be stored.
    pub data_dir: String,
    /// Use an ephemeral db instead of a db in the `data_dir`
    pub use_ephemeral_db: bool,
    /// Data from the Dart Hive DB.
    pub dart_migration_data: Option<DartMigrationData>,
}

/// Discovery Engine endpoint settings.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(derivative::Derivative), derivative(Eq, PartialEq))]
pub struct EndpointConfig {
    /// Page size setting for API.
    pub page_size: usize,
    /// Write-exclusive access to markets list.
    #[serde(skip)]
    #[cfg_attr(test, derivative(PartialEq = "ignore"))]
    pub markets: Arc<RwLock<Vec<Market>>>,
    /// Trusted sources for news queries.
    #[serde(skip)]
    #[cfg_attr(test, derivative(PartialEq = "ignore"))]
    pub trusted_sources: Arc<RwLock<Vec<String>>>,
    /// Sources to exclude for news queries.
    #[serde(skip)]
    #[cfg_attr(test, derivative(PartialEq = "ignore"))]
    pub excluded_sources: Arc<RwLock<Vec<String>>>,
    /// The maximum number of requests to try to reach the number of `min_articles`.
    pub max_requests: usize,
    /// The minimum number of new articles to try to return when updating the stack.
    pub min_articles: usize,
    /// The maximum age of a headline, in days, after which we no longer
    /// want to display them.
    pub max_headline_age_days: usize,
    /// The maximum age of a news article, in days, after which we no longer
    /// want to display them.
    pub max_article_age_days: usize,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            page_size: 100,
            markets: Arc::default(),
            trusted_sources: Arc::default(),
            excluded_sources: Arc::default(),
            max_requests: 5,
            min_articles: 20,
            max_headline_age_days: 3,
            max_article_age_days: 30,
        }
    }
}

impl EndpointConfig {
    pub(crate) fn with_markets(mut self, markets: Vec<Market>) -> Self {
        self.markets = Arc::new(RwLock::new(markets));
        self
    }

    pub(crate) fn with_sources(
        mut self,
        trusted: HashSet<String>,
        excluded: HashSet<String>,
    ) -> Self {
        self.trusted_sources = Arc::new(RwLock::new(trusted.into_iter().collect()));
        self.excluded_sources = Arc::new(RwLock::new(excluded.into_iter().collect()));
        self
    }
}

/// Internal config to allow for configurations within the core without a mirroring outside impl.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct CoreConfig {
    /// The number of taken top key phrases while updating the stacks.
    pub take_top: usize,
    /// The number of top documents per stack to keep while filtering the stacks.
    pub keep_top: usize,
    /// The lower bound of documents per stack at which new items are requested.
    pub request_new: usize,
    /// The number of times to get feed documents after which the stacks are updated without the
    /// limitation of `request_new`.
    pub request_after: usize,
    /// The maximum number of top key phrases extracted from the search term in the deep search.
    pub deep_search_top: usize,
    /// The maximum number of documents returned from the deep search.
    pub deep_search_max: usize,
    /// The minimum cosine similarity wrt the original document below which documents returned from
    /// the deep search are discarded.
    pub deep_search_sim: f32,
    /// The probability for random exploration instead of greedy selection in the MAB.
    pub epsilon: f32,
    /// The maximum number of likes and dislikes after which the MAB parameters are rescaled.
    pub max_reactions: usize,
    /// The value by how much the likes and dislikes are incremented when the MAB parameters are
    /// updated.
    pub incr_reactions: f32,
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
            epsilon: 0.2,
            max_reactions: 10,
            incr_reactions: 1.,
        }
    }
}

/// Configurations for the dynamic stacks.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) struct StackConfig {
    /// The maximum cosine similarity wrt to the closest negative coi below which documents are
    /// retained when the stack is updated.
    pub(crate) max_negative_similarity: f32,
}

impl Default for StackConfig {
    fn default() -> Self {
        Self {
            max_negative_similarity: 0.7,
        }
    }
}

/// Configurations for the exploration stack.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct ExplorationConfig {
    /// The number of candidates.
    pub number_of_candidates: usize,
    /// The maximum number of documents to keep.
    pub max_selected_docs: usize,
    /// The maximum cosine similarity wrt to the closest coi below which documents are retained
    /// when the exploration stack is updated.
    pub max_similarity: f32,
}

impl Default for ExplorationConfig {
    fn default() -> Self {
        Self {
            number_of_candidates: 40,
            max_selected_docs: 20,
            max_similarity: 0.7,
        }
    }
}

/// Configurations for the feed.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FeedConfig {
    /// The maximum number of documents per feed batch.
    pub max_docs_per_batch: usize,
}

impl FeedConfig {
    /// Merges existent values from the DE configuration into this configuration.
    pub(crate) fn merge(&mut self, de_config: &Figment) {
        if let Ok(max_docs_per_batch) = de_config.extract_inner("feed.max_docs_per_batch") {
            self.max_docs_per_batch = max_docs_per_batch;
        }
    }
}

/// Configurations for the search.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchConfig {
    /// The maximum number of documents per search batch.
    pub max_docs_per_batch: usize,
}

impl SearchConfig {
    /// Merges existent values from the DE configuration into this configuration
    pub(crate) fn merge(&mut self, de_config: &Figment) {
        if let Ok(max_docs_per_batch) = de_config.extract_inner("search.max_docs_per_batch") {
            self.max_docs_per_batch = max_docs_per_batch;
        }
    }
}

/// Reads the DE configurations from json.
pub(crate) fn de_config_from_json(json: &str) -> Figment {
    Figment::from(Json::string(json))
}

/// Reads the DE configurations from json and sets defaults for missing fields (if possible).
pub(crate) fn de_config_from_json_with_defaults(json: &str) -> Figment {
    de_config_from_json(json)
        .join(Serialized::default("bert.token_size", 150))
        .join(Serialized::default("coi", CoiConfig::default()))
        .join(Serialized::default("kps", KpsConfig::default()))
        .join(Serialized::default("core", CoreConfig::default()))
        .join(Serialized::default("endpoint", EndpointConfig::default()))
        .join(Serialized::default(
            &format!("stacks.{}", BreakingNews::id()),
            StackConfig::default(),
        ))
        .join(Serialized::default(
            &format!("stacks.{}", Exploration::id()),
            ExplorationConfig::default(),
        ))
        .join(Serialized::default(
            &format!("stacks.{}", PersonalizedNews::id()),
            StackConfig::default(),
        ))
        .join(Serialized::default(
            &format!("stacks.{}", TrustedNews::id()),
            StackConfig::default(),
        ))
}

#[cfg(test)]
mod tests {
    use xayn_ai_test_utils::assert_approx_eq;
    use xayn_discovery_engine_ai::GenericError;

    use super::*;

    // the f32 fields are never NaN by construction
    impl Eq for CoreConfig {}
    impl Eq for ExplorationConfig {}

    impl Default for FeedConfig {
        fn default() -> Self {
            Self {
                max_docs_per_batch: 2,
            }
        }
    }

    impl Default for SearchConfig {
        fn default() -> Self {
            Self {
                max_docs_per_batch: 20,
            }
        }
    }

    #[test]
    fn test_de_config_from_json_default() -> Result<(), GenericError> {
        let de_config = de_config_from_json_with_defaults("{}");
        assert_eq!(de_config.extract_inner::<usize>("bert.token_size")?, 150);
        assert_eq!(
            de_config.extract_inner::<CoiConfig>("coi")?,
            CoiConfig::default(),
        );
        assert_eq!(
            de_config.extract_inner::<KpsConfig>("kps")?,
            KpsConfig::default(),
        );
        assert_eq!(
            de_config.extract_inner::<CoreConfig>("core")?,
            CoreConfig::default(),
        );
        assert_eq!(
            de_config.extract_inner::<EndpointConfig>("endpoint")?,
            EndpointConfig::default(),
        );
        assert!(de_config.extract_inner::<u64>("endpoint.timeout").is_err());
        assert!(de_config.extract_inner::<usize>("endpoint.retry").is_err());
        assert_eq!(
            de_config.extract_inner::<StackConfig>(&format!("stacks.{}", BreakingNews::id()))?,
            StackConfig::default(),
        );
        assert_eq!(
            de_config
                .extract_inner::<ExplorationConfig>(&format!("stacks.{}", Exploration::id()))?,
            ExplorationConfig::default(),
        );
        assert_eq!(
            de_config
                .extract_inner::<StackConfig>(&format!("stacks.{}", PersonalizedNews::id()))?,
            StackConfig::default(),
        );
        assert_eq!(
            de_config.extract_inner::<StackConfig>(&format!("stacks.{}", TrustedNews::id()))?,
            StackConfig::default(),
        );
        Ok(())
    }

    #[test]
    fn test_de_config_from_json_modified() -> Result<(), GenericError> {
        let de_config = de_config_from_json_with_defaults(
            r#"{
                "coi": {
                    "threshold": 0.42
                },
                "kps": {
                    "penalty": [0.99, 0.66, 0.33]
                },
                "bert": {
                    "token_size": 42,
                    "foo": "bar"
                },
                "baz": 0,
                "stacks": {
                    "77cf9280-bb93-4158-b660-8732927e0dcc": {
                        "number_of_candidates": 42,
                        "alpha": 0.42
                    }
                },
                "endpoint": {
                    "timeout": 1234
                }
            }"#,
        );
        assert_eq!(de_config.extract_inner::<usize>("bert.token_size")?, 42);
        assert_eq!(
            de_config.extract_inner::<CoiConfig>("coi")?,
            CoiConfig::default().with_threshold(0.42)?,
        );
        assert_eq!(
            de_config.extract_inner::<KpsConfig>("kps")?,
            KpsConfig::default().with_penalty(&[0.99, 0.66, 0.33])?,
        );
        assert_eq!(
            de_config.extract_inner::<CoreConfig>("core")?,
            CoreConfig::default(),
        );
        assert_eq!(
            de_config.extract_inner::<EndpointConfig>("endpoint")?,
            EndpointConfig::default(),
        );
        assert_eq!(de_config.extract_inner::<u64>("endpoint.timeout")?, 1234);
        assert!(de_config.extract_inner::<usize>("endpoint.retry").is_err());
        assert_eq!(
            de_config
                .extract_inner::<ExplorationConfig>(&format!("stacks.{}", Exploration::id()))?,
            ExplorationConfig {
                number_of_candidates: 42,
                ..ExplorationConfig::default()
            },
        );
        assert_approx_eq!(
            f32,
            de_config.extract_inner::<f32>(&format!("stacks.{}.alpha", Exploration::id()))?,
            0.42,
        );
        assert!(de_config
            .extract_inner::<f32>(&format!("stacks.{}.beta", Exploration::id()))
            .is_err());
        Ok(())
    }
}
