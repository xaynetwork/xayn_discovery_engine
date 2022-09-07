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

//! FFI and logic bindings to `discovery_engine_core`.

#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(
    clippy::future_not_send,
    clippy::pedantic,
    noop_method_call,
    rust_2018_idioms,
    unsafe_code,
    unused_qualifications
)]
#![warn(unreachable_pub, rustdoc::missing_crate_level_docs)]
#![allow(
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate
)]

pub mod async_bindings;
mod tracing;
pub mod types;

use std::path::Path;

use itertools::Itertools;
use xayn_discovery_engine_core::Engine;

#[allow(unsafe_code)]
#[no_mangle]
pub extern "C" fn cfg_feature_storage() -> u8 {
    u8::from(cfg!(feature = "storage"))
}

#[async_bindgen::api(
    use uuid::Uuid;

    use xayn_discovery_engine_ai::Embedding;
    use xayn_discovery_engine_core::{
        document::{Document, HistoricDocument, TimeSpent, TrendingTopic, UserReacted, WeightedSource},
        InitConfig, DartMigrationData
    };
    use xayn_discovery_engine_providers::Market;

    use crate::types::{engine::{SharedEngine, InitializationResult}, search::Search};
)]
impl XaynDiscoveryEngineAsyncFfi {
    /// Initializes the engine.
    #[allow(clippy::box_collection)]
    pub async fn initialize(
        config: Box<InitConfig>,
        state: Option<Box<Vec<u8>>>,
        history: Box<Vec<HistoricDocument>>,
        sources: Box<Vec<WeightedSource>>,
        dart_migration_data: Option<Box<DartMigrationData>>,
    ) -> Box<Result<InitializationResult, String>> {
        tracing::init_tracing(config.log_file.as_deref().map(Path::new));

        Box::new(
            Engine::from_config(
                *config,
                state.as_deref().map(Vec::as_slice),
                &history,
                &sources,
                dart_migration_data.map(|d| *d),
            )
            .await
            .map(|(engine, init_db_hint)| InitializationResult::new(engine, init_db_hint))
            .map_err(|error| error.to_string()),
        )
    }

    /// Configures the running engine.
    pub async fn configure(engine: &SharedEngine, de_config: Box<String>) {
        engine.as_ref().lock().await.configure(&de_config);
    }

    /// Serializes the engine.
    pub async fn serialize(engine: &SharedEngine) -> Box<Result<Vec<u8>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .serialize()
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Sets the markets.
    #[allow(clippy::box_collection)]
    pub async fn set_markets(
        engine: &SharedEngine,
        markets: Box<Vec<Market>>,
        history: Box<Vec<HistoricDocument>>,
        sources: Box<Vec<WeightedSource>>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .set_markets(&history, &sources, *markets)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Gets the next batch of feed documents.
    #[allow(clippy::box_collection)]
    pub async fn feed_next_batch(engine: &SharedEngine) -> Box<Result<Vec<Document>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .feed_next_batch()
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Gets feed documents.
    #[allow(clippy::box_collection)]
    pub async fn get_feed_documents(
        engine: &SharedEngine,
        history: Box<Vec<HistoricDocument>>,
        sources: Box<Vec<WeightedSource>>,
    ) -> Box<Result<Vec<Document>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .get_feed_documents(&history, &sources)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Restores the feed documents, ordered by their global rank (timestamp & local rank).
    pub async fn restore_feed(engine: &SharedEngine) -> Box<Result<Vec<Document>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .restore_feed()
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Deletes the feed documents.
    pub async fn delete_feed_documents(
        engine: &SharedEngine,
        ids: Box<Vec<Uuid>>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .delete_feed_documents(&ids.into_iter().map_into().collect_vec())
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Processes time spent.
    pub async fn time_spent(
        engine: &SharedEngine,
        time_spent: Box<TimeSpent>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .time_spent(*time_spent)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Processes user reaction.
    ///
    /// The history is only required for positive reactions.
    #[allow(clippy::box_collection)]
    pub async fn user_reacted(
        engine: &SharedEngine,
        history: Option<Box<Vec<HistoricDocument>>>,
        sources: Box<Vec<WeightedSource>>,
        reacted: Box<UserReacted>,
    ) -> Box<Result<Document, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .user_reacted(history.as_deref().map(Vec::as_slice), &sources, *reacted)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Perform an active search by query.
    pub async fn search_by_query(
        engine: &SharedEngine,
        query: Box<String>,
        page: u32,
    ) -> Box<Result<Vec<Document>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .search_by_query(query.as_ref(), page)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Perform an active search by topic.
    pub async fn search_by_topic(
        engine: &SharedEngine,
        topic: Box<String>,
        page: u32,
    ) -> Box<Result<Vec<Document>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .search_by_topic(topic.as_ref(), page)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Performs an active search by document id (aka deep search).
    ///
    /// The documents are sorted in descending order wrt their cosine similarity towards the
    /// original search term embedding.
    pub async fn search_by_id(
        engine: &SharedEngine,
        id: Box<Uuid>,
    ) -> Box<Result<Vec<Document>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .search_by_id((*id).into())
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Gets the next batch of the current active search.
    pub async fn search_next_batch(engine: &SharedEngine) -> Box<Result<Vec<Document>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .search_next_batch()
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Restores the current active search, ordered by their global rank (timestamp & local rank).
    pub async fn restore_search(engine: &SharedEngine) -> Box<Result<Vec<Document>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .restore_search()
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Gets the current active search mode and term.
    pub async fn searched_by(engine: &SharedEngine) -> Box<Result<Search, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .searched_by()
                .await
                .map(Into::into)
                .map_err(|error| error.to_string()),
        )
    }

    /// Closes the current active search.
    pub async fn close_search(engine: &SharedEngine) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .close_search()
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Performs a deep search by term and market.
    ///
    /// The documents are sorted in descending order wrt their cosine similarity towards the
    /// original search term embedding.
    pub async fn deep_search(
        engine: &SharedEngine,
        term: Box<String>,
        market: Box<Market>,
        embedding: Box<Embedding>,
    ) -> Box<Result<Vec<Document>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .deep_search(term.as_ref(), market.as_ref(), embedding.as_ref())
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Returns the current trending topics.
    pub async fn trending_topics(engine: &SharedEngine) -> Box<Result<Vec<TrendingTopic>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .trending_topics()
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Sets the trusted sources and updates the stacks based on that.
    pub async fn set_trusted_sources(
        engine: &SharedEngine,
        history: Box<Vec<HistoricDocument>>,
        sources: Box<Vec<WeightedSource>>,
        trusted: Box<Vec<String>>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .set_trusted_sources(&history, &sources, *trusted)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Sets the excluded sources and updates the stacks based on that.
    pub async fn set_excluded_sources(
        engine: &SharedEngine,
        history: Box<Vec<HistoricDocument>>,
        sources: Box<Vec<WeightedSource>>,
        excluded: Box<Vec<String>>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .set_excluded_sources(&history, &sources, *excluded)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Sets a new list of excluded and trusted sources.
    pub async fn set_sources(
        engine: &SharedEngine,
        excluded: Box<Vec<String>>,
        trusted: Box<Vec<String>>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .set_sources(*excluded, *trusted)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Returns the trusted sources.
    pub async fn get_trusted_sources(engine: &SharedEngine) -> Box<Result<Vec<String>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .trusted_sources()
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Returns the excluded sources.
    pub async fn get_excluded_sources(engine: &SharedEngine) -> Box<Result<Vec<String>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .excluded_sources()
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Adds a trusted source.
    pub async fn add_trusted_source(
        engine: &SharedEngine,
        trusted: Box<String>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .add_trusted_source(*trusted)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Removes a trusted source.
    pub async fn remove_trusted_source(
        engine: &SharedEngine,
        trusted: Box<String>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .remove_trusted_source(*trusted)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Adds an excluded source.
    pub async fn add_excluded_source(
        engine: &SharedEngine,
        excluded: Box<String>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .add_excluded_source(*excluded)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Removes an excluded source.
    pub async fn remove_excluded_source(
        engine: &SharedEngine,
        excluded: Box<String>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .remove_excluded_source(*excluded)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Disposes the engine.
    pub async fn dispose(engine: Box<SharedEngine>) {
        engine.as_ref().as_ref().lock().await;
    }

    /// Reset the AI state of this engine
    pub async fn reset_ai(engine: &SharedEngine) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .reset_ai()
                .await
                .map_err(|error| error.to_string()),
        )
    }
}
