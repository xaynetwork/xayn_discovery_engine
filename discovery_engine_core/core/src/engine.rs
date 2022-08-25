// Copyright 2021 Xayn AG
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

#[cfg(feature = "storage")]
use std::path::PathBuf;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    iter::once,
    mem::replace,
};

use cfg_if::cfg_if;
use displaydoc::Display;
use futures::future::join_all;
use itertools::{chain, Itertools};
use ndarray::Array;
use rayon::iter::{Either, IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, Level};

use xayn_discovery_engine_ai::{
    cosine_similarity,
    nan_safe_f32_cmp,
    CoiSystem,
    CoiSystemConfig,
    CoiSystemState,
    Embedding,
    GenericError,
    KeyPhrase,
    UserInterests,
};
use xayn_discovery_engine_bert::{AveragePooler, SMBert, SMBertConfig};
use xayn_discovery_engine_kpe::{Config as KpeConfig, Pipeline as KPE};
use xayn_discovery_engine_providers::{
    clean_query,
    Filter,
    GenericArticle,
    HeadlinesQuery,
    Market,
    NewsQuery,
    Providers,
    RankLimit,
    TrendingTopic as BingTopic,
    TrendingTopicsQuery,
};
use xayn_discovery_engine_tokenizer::{AccentChars, CaseChars};

#[cfg(feature = "storage")]
use crate::storage::{self, sqlite::SqliteStorage, BoxedStorage, Storage};
use crate::{
    config::{
        de_config_from_json,
        de_config_from_json_with_defaults,
        CoreConfig,
        EndpointConfig,
        ExplorationConfig,
        FeedConfig,
        InitConfig,
        SearchConfig,
        StackConfig,
    },
    document::{
        self,
        Document,
        HistoricDocument,
        TimeSpent,
        TrendingTopic,
        UserReacted,
        UserReaction,
        WeightedSource,
    },
    mab::{self, BetaSampler, Bucket, SelectionIter, UniformSampler},
    stack::{
        self,
        exploration::Stack as Exploration,
        filters::{
            filter_semantically,
            ArticleFilter,
            Criterion,
            DuplicateFilter,
            MalformedFilter,
            SemanticFilterConfig,
        },
        BoxedOps,
        BreakingNews,
        Data as StackData,
        Id as StackId,
        Id,
        NewItemsError,
        Ops,
        PersonalizedNews,
        Stack,
        TrustedNews,
    },
};

/// Discovery engine errors.
#[derive(Error, Debug, Display)]
pub enum Error {
    /// Failed to serialize internal state of the engine: {0}.
    Serialization(#[source] GenericError),

    /// Failed to deserialize internal state to create the engine: {0}.
    Deserialization(#[source] bincode::Error),

    /// No operations on stack were provided.
    NoStackOps,

    /// Invalid stack: {0}.
    InvalidStack(#[source] stack::Error),

    /// Invalid stack id: {0}.
    InvalidStackId(StackId),

    /// An operation on a stack failed: {0}.
    StackOpFailed(#[source] stack::Error),

    /// Error while selecting the documents to return: {0}.
    Selection(#[from] mab::Error),

    /// Error while using the ranker: {0}
    Ranker(#[from] GenericError),

    /// Error while creating document: {0}.
    Document(#[source] document::Error),

    /// Error while querying with client: {0}.
    Client(#[source] GenericError),

    /// Invalid search term.
    InvalidTerm,

    /// List of errors/warnings: {0:?}.
    Errors(Vec<Error>),

    #[cfg(feature = "storage")]
    /// Storage error: {0}.
    Storage(#[from] storage::Error),

    /// Provider error: {0}
    ProviderError(#[source] xayn_discovery_engine_providers::Error),
}

/// Discovery Engine.
pub struct Engine {
    // configs
    pub(crate) endpoint_config: EndpointConfig,
    pub(crate) core_config: CoreConfig,
    pub(crate) feed_config: FeedConfig,
    pub(crate) search_config: SearchConfig,
    request_after: usize,

    // systems
    smbert: SMBert,
    pub(crate) coi: CoiSystem,
    kpe: KPE,
    providers: Providers,

    // states
    stacks: RwLock<HashMap<StackId, Stack>>,
    pub(crate) exploration_stack: Exploration,
    state: CoiSystemState,
    #[cfg(feature = "storage")]
    storage: BoxedStorage,
}

impl Engine {
    /// Creates a discovery [`Engine`].
    ///
    /// The engine only keeps in its state data related to the current [`BoxedOps`].
    /// Data related to missing operations will be dropped.
    #[allow(clippy::too_many_arguments)]
    async fn new(
        endpoint_config: EndpointConfig,
        core_config: CoreConfig,
        mut stack_config: HashMap<StackId, StackConfig>,
        exploration_config: ExplorationConfig,
        feed_config: FeedConfig,
        search_config: SearchConfig,
        smbert: SMBert,
        coi: CoiSystem,
        kpe: KPE,
        state: CoiSystemState,
        history: &[HistoricDocument],
        sources: &[WeightedSource],
        mut stack_data: HashMap<StackId, StackData>,
        stack_ops: Vec<BoxedOps>,
        #[cfg(feature = "storage")] storage: BoxedStorage,
        providers: Providers,
    ) -> Result<Self, Error> {
        if stack_ops.is_empty() {
            return Err(Error::NoStackOps);
        }

        let stacks = stack_ops
            .into_iter()
            .map(|ops| {
                let id = ops.id();
                let data = stack_data.remove(&id).unwrap_or_default();
                let config = stack_config.remove(&id).unwrap_or_default();
                Stack::new(data, ops, config).map(|stack| (id, stack))
            })
            .collect::<Result<HashMap<_, _>, _>>()
            .map(RwLock::new)
            .map_err(Error::InvalidStack)?;

        let exploration_stack = Exploration::new(
            stack_data.remove(&Exploration::id()).unwrap_or_default(),
            exploration_config,
        )
        .map_err(Error::InvalidStack)?;

        let mut engine = Self {
            endpoint_config,
            core_config,
            feed_config,
            search_config,
            request_after: 0,
            smbert,
            coi,
            kpe,
            providers,
            stacks,
            exploration_stack,
            state,
            #[cfg(feature = "storage")]
            storage,
        };

        // we don't want to fail initialization if there are network problems
        engine
            .update_stacks_for_all_markets(history, sources, usize::MAX)
            .await
            .ok();

        Ok(engine)
    }

    /// Creates a discovery [`Engine`] from a configuration and optional state.
    #[allow(clippy::too_many_lines)]
    pub async fn from_config(
        config: InitConfig,
        // TODO: change this to a boolean flag after DB migration
        state: Option<&[u8]>,
        history: &[HistoricDocument],
        sources: &[WeightedSource],
    ) -> Result<Self, Error> {
        // read the configs
        let de_config =
            de_config_from_json_with_defaults(config.de_config.as_deref().unwrap_or("{}"));
        let endpoint_config = de_config
            .extract_inner::<EndpointConfig>("endpoint")
            .map_err(|err| Error::Ranker(err.into()))?
            .with_init_config(&config);
        let core_config = de_config
            .extract_inner("core")
            .map_err(|err| Error::Ranker(err.into()))?;
        let feed_config = FeedConfig {
            max_docs_per_batch: config.max_docs_per_feed_batch as usize,
        };
        let search_config = SearchConfig {
            max_docs_per_batch: config.max_docs_per_search_batch as usize,
        };
        let exploration_config = de_config
            .extract_inner(&format!("stacks.{}", Exploration::id()))
            .map_err(|err| Error::Ranker(err.into()))?;
        let provider_config =
            config.to_provider_config(endpoint_config.timeout, endpoint_config.retry);

        // build the systems
        let smbert = SMBertConfig::from_files(&config.smbert_vocab, &config.smbert_model)
            .map_err(|err| Error::Ranker(err.into()))?
            .with_token_size(
                de_config
                    .extract_inner("smbert.token_size")
                    .map_err(|err| Error::Ranker(err.into()))?,
            )
            .map_err(|err| Error::Ranker(err.into()))?
            .with_accents(AccentChars::Cleanse)
            .with_case(CaseChars::Lower)
            .with_pooling::<AveragePooler>()
            .build()
            .map_err(GenericError::from)?;
        let coi = de_config
            .extract::<CoiSystemConfig>()
            .map_err(|err| Error::Ranker(err.into()))?
            .build();
        let kpe = KpeConfig::from_files(
            &config.kpe_vocab,
            &config.kpe_model,
            &config.kpe_cnn,
            &config.kpe_classifier,
        )
        .map_err(|err| Error::Ranker(err.into()))?
        .with_token_size(
            de_config
                .extract_inner("kpe.token_size")
                .map_err(|err| Error::Ranker(err.into()))?,
        )
        .map_err(|err| Error::Ranker(err.into()))?
        .with_accents(AccentChars::Cleanse)
        .with_case(CaseChars::Keep)
        .build()
        .map_err(GenericError::from)?;
        let providers = Providers::new(provider_config).map_err(Error::ProviderError)?;

        // initialize the states
        #[cfg(feature = "storage")]
        let storage = {
            let sqlite_uri = if config.use_in_memory_db {
                "sqlite::memory:".into()
            } else {
                let db_path = PathBuf::from(&config.data_dir).join("db.sqlite");
                format!("sqlite:{}", db_path.display())
            };
            let storage = SqliteStorage::connect(&sqlite_uri).await?;
            storage.init_database().await?;
            Box::new(storage)
        };
        let stack_ops = vec![
            Box::new(BreakingNews::new(
                &endpoint_config,
                providers.headlines.clone(),
            )) as BoxedOps,
            Box::new(TrustedNews::new(
                &endpoint_config,
                providers.trusted_headlines.clone(),
            )) as BoxedOps,
            Box::new(PersonalizedNews::new(
                &endpoint_config,
                providers.similar_news.clone(),
            )) as BoxedOps,
        ];
        let stack_config = stack_ops
            .iter()
            .try_fold(HashMap::new(), |mut configs, ops| {
                let id = ops.id();
                de_config
                    .extract_inner(&format!("stacks.{}", id))
                    .map(|config| {
                        configs.insert(id, config);
                        configs
                    })
                    .map_err(|err| Error::Ranker(err.into()))
            })?;
        #[cfg(feature = "storage")]
        let (mut stack_data, state) = if state.is_some() {
            storage
                .state()
                .fetch()
                .await?
                .map(|state| SerializedState::deserialize(&state))
                .transpose()?
        } else {
            storage.state().clear().await?;
            None
        }
        .unwrap_or_default();
        #[cfg(not(feature = "storage"))]
        let (mut stack_data, state) = state
            .map(SerializedState::deserialize)
            .transpose()?
            .unwrap_or_default();
        for id in stack_ops.iter().map(Ops::id).chain(once(Exploration::id())) {
            if let Ok(alpha) = de_config.extract_inner::<f32>(&format!("stacks.{id}.alpha")) {
                stack_data.entry(id).or_default().alpha = alpha;
            }
            if let Ok(beta) = de_config.extract_inner::<f32>(&format!("stacks.{id}.beta")) {
                stack_data.entry(id).or_default().beta = beta;
            }
        }

        Self::new(
            endpoint_config,
            core_config,
            stack_config,
            exploration_config,
            feed_config,
            search_config,
            smbert,
            coi,
            kpe,
            state,
            history,
            sources,
            stack_data,
            stack_ops,
            #[cfg(feature = "storage")]
            storage,
            providers,
        )
        .await
    }

    /// Configures the running engine.
    pub fn configure(&mut self, de_config: &str) {
        let de_config = de_config_from_json(de_config);
        self.feed_config.merge(&de_config);
        self.search_config.merge(&de_config);
    }

    async fn update_stacks_for_all_markets(
        &mut self,
        history: &[HistoricDocument],
        sources: &[WeightedSource],
        request_new: usize,
    ) -> Result<(), Error> {
        let markets = self.endpoint_config.markets.read().await;
        let mut stacks = self.stacks.write().await;

        update_stacks(
            &mut stacks,
            &mut self.exploration_stack,
            &self.smbert,
            &self.coi,
            &mut self.state,
            history,
            sources,
            self.core_config.take_top,
            self.core_config.keep_top,
            request_new,
            &markets,
        )
        .await
    }

    /// Serializes the state of the `Engine` and `Ranker` state.
    // TODO: remove the ffi for this method and reduce its visibility after DB migration
    pub async fn serialize(&self) -> Result<Vec<u8>, Error> {
        let stacks = self.stacks.read().await;
        let mut stacks_data = stacks
            .iter()
            .map(|(id, stack)| (id, &stack.data))
            .collect::<HashMap<_, _>>();
        let exploration_stack_id = Exploration::id();
        stacks_data.insert(&exploration_stack_id, &self.exploration_stack.data);

        let stacks = bincode::serialize(&stacks_data)
            .map(SerializedStackState)
            .map_err(|err| Error::Serialization(err.into()))?;

        let coi = self
            .state
            .serialize()
            .map(SerializedCoiSystemState)
            .map_err(Error::Serialization)?;

        let state = bincode::serialize(&SerializedState { stacks, coi })
            .map_err(|err| Error::Serialization(err.into()))?;

        #[cfg(feature = "storage")]
        {
            self.storage.state().store(state).await?;
            Ok(Vec::new())
        }

        #[cfg(not(feature = "storage"))]
        Ok(state)
    }

    /// Updates the markets configuration.
    ///
    /// Also resets and updates all stacks.
    pub async fn set_markets(
        &mut self,
        history: &[HistoricDocument],
        sources: &[WeightedSource],
        new_markets: Vec<Market>,
    ) -> Result<(), Error> {
        let mut markets_guard = self.endpoint_config.markets.write().await;
        let mut old_markets = replace(&mut *markets_guard, new_markets);
        old_markets.retain(|market| !markets_guard.contains(market));
        CoiSystem::remove_key_phrases(&old_markets, &mut self.state.key_phrases);
        drop(markets_guard);

        self.clear_stack_data().await;

        self.update_stacks_for_all_markets(history, sources, self.core_config.request_new)
            .await
    }

    /// Clears the data of all stacks
    async fn clear_stack_data(&mut self) {
        let mut stacks = self.stacks.write().await;
        for stack in stacks.values_mut() {
            stack.data = StackData::default();
        }
        drop(stacks); // guard
        self.exploration_stack.data = StackData::default();
    }

    /// Gets the next batch of feed documents.
    #[instrument(skip(self))]
    #[cfg_attr(not(feature = "storage"), allow(clippy::unused_async))]
    pub async fn feed_next_batch(
        &mut self,
        sources: &[WeightedSource],
    ) -> Result<Vec<Document>, Error> {
        #[cfg(feature = "storage")]
        {
            let history = self.storage.fetch_history().await?;

            // TODO: merge `get_feed_documents()` into this method after DB migration
            self.get_feed_documents(&history, sources).await
        }

        #[cfg(not(feature = "storage"))]
        unimplemented!("requires 'storage' feature")
    }

    /// Gets the next batch of feed documents.
    #[instrument(skip(self, history))]
    pub async fn get_feed_documents(
        &mut self,
        history: &[HistoricDocument],
        sources: &[WeightedSource],
    ) -> Result<Vec<Document>, Error> {
        let request_new = (self.request_after < self.core_config.request_after)
            .then(|| self.core_config.request_new)
            .unwrap_or(usize::MAX);

        self.update_stacks_for_all_markets(history, sources, request_new)
            .await?;

        self.request_after = (self.request_after + 1) % self.core_config.request_after;

        let mut stacks = self.stacks.write().await;
        let all_stacks = chain!(
            stacks
                .values_mut()
                .map(|stack| stack as &mut (dyn Bucket<Document> + Send)),
            once(&mut self.exploration_stack as _),
        );

        let documents = SelectionIter::new(
            self.core_config.epsilon,
            UniformSampler,
            UniformSampler,
            BetaSampler,
            all_stacks,
        )?
        .select(self.feed_config.max_docs_per_batch)?;
        drop(stacks); // guard
        for document in &documents {
            debug!(
                document = %document.id,
                stack = %document.stack_id,
                title = %document.resource.title,
            );
        }

        #[cfg(feature = "storage")]
        {
            let stack_ids = documents.iter().map(|doc| (doc.id, doc.stack_id)).collect();
            let documents = documents.iter().cloned().map_into().collect_vec();
            self.storage
                .feed()
                .store_documents(&documents, &stack_ids)
                .await?;
            self.serialize().await?;
        }

        Ok(documents)
    }

    /// Restores the feed documents, ordered by their global rank (timestamp & local rank).
    // TODO: rename methods to `fed()` and adjust events & docs accordingly after DB migration
    #[cfg_attr(not(feature = "storage"), allow(clippy::unused_async))]
    pub async fn restore_feed(&self) -> Result<Vec<Document>, Error> {
        #[cfg(feature = "storage")]
        {
            self.storage
                .feed()
                .fetch()
                .await
                .map(|documents| documents.into_iter().map_into().collect())
                .map_err(Into::into)
        }

        #[cfg(not(feature = "storage"))]
        unimplemented!("requires 'storage' feature")
    }

    /// Deletes the feed documents.
    #[cfg_attr(
        not(feature = "storage"),
        allow(unused_variables, clippy::unused_async)
    )]
    pub async fn delete_feed_documents(&self, ids: &[document::Id]) -> Result<(), Error> {
        #[cfg(feature = "storage")]
        {
            self.storage.feed().delete_documents(ids).await?;

            Ok(())
        }

        #[cfg(not(feature = "storage"))]
        unimplemented!("requires 'storage' feature")
    }

    /// Process the feedback about the user spending some time on a document.
    pub async fn time_spent(&mut self, time_spent: TimeSpent) -> Result<(), Error> {
        cfg_if! {
            if #[cfg(feature = "storage")] {
                use storage::models::TimeSpentDocumentView;

                let TimeSpent {
                    id,
                    view_time,
                    view_mode,
                    // dummy values when storage is enabled
                    ..
                } = time_spent;

                let TimeSpentDocumentView {
                    smbert_embedding,
                    aggregated_view_time,
                    last_reaction,
                } = self
                    .storage
                    .feedback()
                    .update_time_spent(id, view_mode, view_time)
                    .await?;

                let last_reaction = last_reaction.unwrap_or(UserReaction::Neutral);
            } else {
                let TimeSpent {
                    id: _,
                    view_time: aggregated_view_time,
                    view_mode: _,
                    smbert_embedding,
                    reaction: last_reaction,
                } = time_spent;
            }
        };

        if let UserReaction::Positive | UserReaction::Neutral = last_reaction {
            CoiSystem::log_document_view_time(
                &mut self.state.user_interests.positive,
                &smbert_embedding,
                aggregated_view_time,
            );
        }

        rank_stacks(
            self.stacks.write().await.values_mut(),
            &mut self.exploration_stack,
            &self.coi,
            &self.state.user_interests,
        );

        #[cfg(feature = "storage")]
        self.serialize().await?;

        Ok(())
    }

    /// Process the feedback about the user reacting to a document.
    ///
    /// The history is only required for positive reactions.
    #[instrument(skip(self), level = "debug")]
    pub async fn user_reacted(
        &mut self,
        history: Option<&[HistoricDocument]>,
        sources: &[WeightedSource],
        reacted: UserReacted,
    ) -> Result<Document, Error> {
        let reaction = reacted.reaction;
        cfg_if! {
            if #[cfg(feature="storage")] {
                let document: Document = self.storage
                    .feedback()
                    .update_user_reaction(reacted.id, reaction).await?
                    .into();
            } else {
                use chrono::Utc;
                use url::Url;

                use crate::document::NewsResource;

                let document = Document {
                    id: reacted.id,
                    stack_id: reacted.stack_id,
                    smbert_embedding: reacted.smbert_embedding,
                    reaction: None,
                    resource: NewsResource {
                        title: reacted.title,
                        snippet: reacted.snippet,
                        url: Url::parse("https://foobar.invalid").unwrap(),
                        source_domain: "foobar.invalid".into(),
                        date_published: Utc::now().naive_utc(),
                        image: None,
                        rank: 0,
                        score: None,
                        country: reacted.market.country_code,
                        language: reacted.market.lang_code,
                        topic: "invalid".into()
                    }
                };
            }
        }

        let market = Market::new(&document.resource.language, &document.resource.country);

        let mut stacks = self.stacks.write().await;

        // update relevance of stack if the reacted document belongs to one
        if !document.stack_id.is_nil() {
            if let Some(stack) = stacks.get_mut(&document.stack_id) {
                stack.update_relevance(
                    reaction,
                    self.core_config.max_reactions,
                    self.core_config.incr_reactions,
                );
            } else if document.stack_id == Exploration::id() {
                self.exploration_stack.update_relevance(
                    reaction,
                    self.core_config.max_reactions,
                    self.core_config.incr_reactions,
                );
            } else {
                return Err(Error::InvalidStackId(document.stack_id));
            }
        };

        match reaction {
            UserReaction::Positive => {
                let smbert = &self.smbert;
                self.coi.log_positive_user_reaction(
                    &mut self.state.user_interests.positive,
                    &market,
                    &mut self.state.key_phrases,
                    &document.smbert_embedding,
                    &[document.resource.snippet_or_title().to_string()],
                    |words| smbert.run(words).map_err(Into::into),
                );
            }
            UserReaction::Negative => self.coi.log_negative_user_reaction(
                &mut self.state.user_interests.negative,
                &document.smbert_embedding,
            ),
            UserReaction::Neutral => {}
        }
        debug!(user_interests = ?self.state.user_interests);

        rank_stacks(
            stacks.values_mut(),
            &mut self.exploration_stack,
            &self.coi,
            &self.state.user_interests,
        );
        if let UserReaction::Positive = reaction {
            if let Some(history) = history {
                update_stacks(
                    &mut stacks,
                    &mut self.exploration_stack,
                    &self.smbert,
                    &self.coi,
                    &mut self.state,
                    history,
                    sources,
                    self.core_config.take_top,
                    self.core_config.keep_top,
                    usize::MAX,
                    &[market.clone()],
                )
                .await?;
                self.request_after = 0;
            } else {
                return Err(Error::StackOpFailed(stack::Error::NoHistory));
            }
        }
        drop(stacks); // guard

        #[cfg(feature = "storage")]
        self.serialize().await?;

        Ok(document)
    }

    /// Perform an active search by query.
    pub async fn search_by_query(&self, query: &str, page: u32) -> Result<Vec<Document>, Error> {
        let query = clean_query(query);
        if query.trim().is_empty() {
            return Err(Error::InvalidTerm);
        }

        #[cfg(feature = "storage")]
        match self.storage.search().fetch().await {
            Ok(_) => return Err(storage::Error::OpenSearch.into()),
            Err(storage::Error::NoSearch) => { /* continue */ }
            Err(error) => return Err(error.into()),
        };

        let filter = Filter::default().add_keyword(&query);
        let documents = self
            .active_search(
                SearchBy::Query(Cow::Owned(filter)),
                page,
                self.search_config.max_docs_per_batch,
            )
            .await?;

        #[cfg(feature = "storage")]
        {
            #[allow(clippy::cast_possible_truncation)] // originally u32 in InitConfig
            let search = storage::models::Search {
                search_by: storage::models::SearchBy::Query,
                search_term: query,
                paging: storage::models::Paging {
                    size: self.search_config.max_docs_per_batch as u32,
                    next_page: page,
                },
            };
            let documents = documents.iter().cloned().map_into().collect_vec();
            self.storage
                .search()
                .store_new_search(&search, &documents)
                .await?;
            self.serialize().await?;
        }

        Ok(documents)
    }

    /// Perform an active search by topic.
    pub async fn search_by_topic(&self, topic: &str, page: u32) -> Result<Vec<Document>, Error> {
        if topic.trim().is_empty() {
            return Err(Error::InvalidTerm);
        }

        #[cfg(feature = "storage")]
        match self.storage.search().fetch().await {
            Ok(_) => return Err(storage::Error::OpenSearch.into()),
            Err(storage::Error::NoSearch) => { /* continue */ }
            Err(error) => return Err(error.into()),
        };

        let documents = self
            .active_search(
                SearchBy::Topic(topic.into()),
                page,
                self.search_config.max_docs_per_batch,
            )
            .await?;

        #[cfg(feature = "storage")]
        {
            #[allow(clippy::cast_possible_truncation)] // originally u32 in InitConfig
            let search = storage::models::Search {
                search_by: storage::models::SearchBy::Topic,
                search_term: topic.into(),
                paging: storage::models::Paging {
                    size: self.search_config.max_docs_per_batch as u32,
                    next_page: page,
                },
            };
            let documents = documents.iter().cloned().map_into().collect_vec();
            self.storage
                .search()
                .store_new_search(&search, &documents)
                .await?;
            self.serialize().await?;
        }

        Ok(documents)
    }

    /// Performs an active search by document id (aka deep search).
    ///
    /// The documents are sorted in descending order wrt their cosine similarity towards the
    /// original search term embedding.
    #[cfg_attr(
        not(feature = "storage"),
        allow(unused_variables, clippy::unused_async)
    )]
    pub async fn search_by_id(&self, id: document::Id) -> Result<Vec<Document>, Error> {
        #[cfg(feature = "storage")]
        {
            let document = self.storage.search().get_document(id).await?;

            // TODO: merge `deep_search()` into this method after DB migration
            self.deep_search(
                document.snippet_or_title(),
                &document.news_resource.market,
                &document.embedding,
            )
            .await
        }

        #[cfg(not(feature = "storage"))]
        unimplemented!("requires 'storage' feature")
    }

    /// Gets the next batch of the current active search.
    #[cfg_attr(not(feature = "storage"), allow(clippy::unused_async))]
    pub async fn search_next_batch(&self) -> Result<Vec<Document>, Error> {
        #[cfg(feature = "storage")]
        {
            let (search, _) = self.storage.search().fetch().await?;
            let by = match search.search_by {
                storage::models::SearchBy::Query => SearchBy::Query(Cow::Owned(
                    Filter::default().add_keyword(&search.search_term),
                )),
                storage::models::SearchBy::Topic => SearchBy::Topic(search.search_term.into()),
            };
            let page_number = search.paging.next_page + 1;
            let documents = self
                .active_search(by, page_number, search.paging.size as usize)
                .await?;

            self.storage
                .search()
                .store_next_page(
                    page_number,
                    documents
                        .iter()
                        .cloned()
                        .map_into()
                        .collect_vec()
                        .as_slice(),
                )
                .await?;
            self.serialize().await?;

            Ok(documents)
        }

        #[cfg(not(feature = "storage"))]
        unimplemented!("requires 'storage' feature")
    }

    /// Restores the current active search, ordered by their global rank (timestamp & local rank).
    // TODO: rename methods to `searched()` and adjust events & docs accordingly after DB migration
    #[cfg_attr(not(feature = "storage"), allow(clippy::unused_async))]
    pub async fn restore_search(&self) -> Result<Vec<Document>, Error> {
        #[cfg(feature = "storage")]
        {
            let (_, documents) = self.storage.search().fetch().await?;
            let documents = documents.into_iter().map_into().collect();

            Ok(documents)
        }

        #[cfg(not(feature = "storage"))]
        unimplemented!("requires 'storage' feature")
    }

    /// Gets the current active search mode and term.
    #[cfg_attr(not(feature = "storage"), allow(clippy::unused_async))]
    pub async fn searched_by(&self) -> Result<SearchBy<'_>, Error> {
        #[cfg(feature = "storage")]
        {
            let (search, _) = self.storage.search().fetch().await?;
            let search = match search.search_by {
                storage::models::SearchBy::Query => SearchBy::Query(Cow::Owned(
                    Filter::default().add_keyword(&search.search_term),
                )),
                storage::models::SearchBy::Topic => SearchBy::Topic(search.search_term.into()),
            };

            Ok(search)
        }

        #[cfg(not(feature = "storage"))]
        unimplemented!("requires 'storage' feature")
    }

    /// Closes the current active search.
    #[cfg_attr(not(feature = "storage"), allow(clippy::unused_async))]
    pub async fn close_search(&self) -> Result<(), Error> {
        #[cfg(feature = "storage")]
        {
            if self.storage.search().clear().await? {
                Ok(())
            } else {
                Err(Error::Storage(storage::Error::NoSearch))
            }
        }

        #[cfg(not(feature = "storage"))]
        unimplemented!("requires 'storage' feature")
    }

    async fn active_search(
        &self,
        by: SearchBy<'_>,
        page: u32,
        page_size: usize,
    ) -> Result<Vec<Document>, Error> {
        let mut errors = Vec::new();
        let mut articles = Vec::new();

        let markets = self.endpoint_config.markets.read().await;
        let scaled_page_size = page_size / markets.len() + 1;
        let excluded_sources = self.endpoint_config.excluded_sources.read().await.clone();
        for market in markets.iter() {
            let query_result = match &by {
                SearchBy::Query(filter) => {
                    let news_query = NewsQuery {
                        filter: filter.as_ref(),
                        max_age_days: None,
                        market,
                        page_size: scaled_page_size,
                        page: page as usize,
                        rank_limit: RankLimit::Unlimited,
                        excluded_sources: &excluded_sources,
                    };
                    self.providers.news.query_news(&news_query).await
                }
                SearchBy::Topic(topic) => {
                    let headlines_query = HeadlinesQuery {
                        trusted_sources: &[],
                        topic: Some(topic),
                        max_age_days: None,
                        market,
                        page_size: scaled_page_size,
                        page: page as usize,
                        rank_limit: RankLimit::Unlimited,
                        excluded_sources: &excluded_sources,
                    };
                    self.providers
                        .headlines
                        .query_headlines(&headlines_query)
                        .await
                }
            };
            query_result.map_or_else(
                |err| errors.push(Error::Client(err.into())),
                |batch| articles.extend(batch),
            );
        }

        let (mut documents, article_errors) = documentify_articles(
            StackId::nil(), // these documents are not associated with a stack
            &self.smbert,
            articles,
        );
        errors.extend(article_errors);

        self.coi.rank(&mut documents, &self.state.user_interests);
        if documents.is_empty() && !errors.is_empty() {
            Err(Error::Errors(errors))
        } else {
            documents.truncate(page_size);
            Ok(documents)
        }
    }

    /// Performs a deep search by term and market.
    ///
    /// The documents are sorted in descending order wrt their cosine similarity towards the
    /// original search term embedding.
    pub async fn deep_search(
        &self,
        term: &str,
        market: &Market,
        embedding: &Embedding,
    ) -> Result<Vec<Document>, Error> {
        let key_phrases = self
            .kpe
            .run(clean_query(term))
            .map_err(GenericError::from)?;
        if key_phrases.is_empty() {
            return Ok(Vec::new());
        }

        let excluded_sources = &self.endpoint_config.excluded_sources.read().await.clone();
        let filter = &key_phrases
            .iter()
            .take(self.core_config.deep_search_top)
            .fold(Filter::default(), |filter, key_phrase| {
                filter.add_keyword(key_phrase)
            });
        let query = NewsQuery {
            market,
            page_size: self.core_config.deep_search_max,
            page: 1,
            rank_limit: RankLimit::Unlimited,
            excluded_sources,
            filter,
            max_age_days: None,
        };

        let articles = self
            .providers
            .news
            .query_news(&query)
            .await
            .map_err(|error| Error::Client(error.into()))?;
        let articles = MalformedFilter::apply(&[], &[], articles)?;
        let (documents, errors) = documentify_articles(
            StackId::nil(), // these documents are not associated with a stack
            &self.smbert,
            articles,
        );

        // only return an error if all articles failed
        if documents.is_empty() && !errors.is_empty() {
            return Err(Error::Errors(errors));
        }

        let mut documents = documents
            .into_iter()
            .filter_map(|document| {
                let similarity =
                    cosine_similarity(embedding.view(), document.smbert_embedding.view());
                (similarity > self.core_config.deep_search_sim).then(|| (similarity, document))
            })
            .collect_vec();
        documents.sort_unstable_by(|(this, _), (other, _)| nan_safe_f32_cmp(this, other).reverse());
        let documents = documents
            .into_iter()
            .map(|(_, document)| document)
            .collect();

        Ok(documents)
    }

    /// Returns the current trending topics.
    pub async fn trending_topics(&mut self) -> Result<Vec<TrendingTopic>, Error> {
        let mut errors = Vec::new();
        let mut topics = Vec::new();

        let markets = self.endpoint_config.markets.read().await;
        for market in markets.iter() {
            let query = TrendingTopicsQuery { market };
            match self
                .providers
                .trending_topics
                .query_trending_topics(&query)
                .await
            {
                Ok(batch) => topics.extend(batch),
                Err(err) => errors.push(Error::Client(err.into())),
            };
        }

        let (mut topics, topic_errors) = documentify_topics(&self.smbert, topics);
        errors.extend(topic_errors);

        self.coi.rank(&mut topics, &self.state.user_interests);
        if topics.is_empty() && !errors.is_empty() {
            Err(Error::Errors(errors))
        } else {
            Ok(topics)
        }
    }

    async fn filter_excluded_sources_for_all_stacks(&mut self, sources: &HashSet<String>) {
        let mut stacks = self.stacks.write().await;
        for stack in stacks.values_mut() {
            stack.prune_by_excluded_sources(sources);
        }
        drop(stacks); // guard
        self.exploration_stack.prune_by_excluded_sources(sources);
    }

    /// Sets a new list of trusted sources.
    pub async fn set_trusted_sources(
        &mut self,
        history: &[HistoricDocument],
        sources: &[WeightedSource],
        trusted: Vec<String>,
    ) -> Result<(), Error> {
        *self.endpoint_config.trusted_sources.write().await = trusted;
        self.update_stacks_for_all_markets(history, sources, self.core_config.request_new)
            .await
    }

    /// Sets a new list of excluded sources.
    pub async fn set_excluded_sources(
        &mut self,
        history: &[HistoricDocument],
        sources: &[WeightedSource],
        excluded: Vec<String>,
    ) -> Result<(), Error> {
        let exclusion_set = excluded.iter().cloned().collect::<HashSet<_>>();
        *self.endpoint_config.excluded_sources.write().await = excluded;

        self.filter_excluded_sources_for_all_stacks(&exclusion_set)
            .await;
        self.update_stacks_for_all_markets(history, sources, self.core_config.request_new)
            .await
    }

    /// Sets a new list of excluded and trusted sources.
    #[cfg_attr(not(feature = "storage"), allow(unused_variables))]
    pub async fn set_sources(
        &mut self,
        sources: &[WeightedSource],
        excluded: Vec<String>,
        trusted: Vec<String>,
    ) -> Result<(), Error> {
        #[cfg(feature = "storage")]
        {
            let trusted_set = trusted.iter().cloned().collect::<HashSet<_>>();
            let current_trusted = self.storage.source_preference().fetch_trusted().await?;
            let trusted_changed = trusted_set != current_trusted;

            let excluded_set = excluded.iter().cloned().collect::<HashSet<_>>();
            let current_excluded = self.storage.source_preference().fetch_excluded().await?;
            let excluded_changed = excluded_set != current_excluded;

            if !trusted_changed && !excluded_changed {
                return Ok(());
            }

            if trusted_changed {
                self.storage
                    .source_preference()
                    .set_trusted(&trusted_set)
                    .await?;
                *self.endpoint_config.trusted_sources.write().await = trusted;
            }

            if excluded_changed {
                self.storage
                    .source_preference()
                    .set_excluded(&excluded_set)
                    .await?;
                *self.endpoint_config.excluded_sources.write().await = excluded;
                self.filter_excluded_sources_for_all_stacks(&excluded_set)
                    .await;
            }

            let history = self.storage.fetch_history().await?;
            self.update_stacks_for_all_markets(&history, sources, self.core_config.request_new)
                .await
        }

        #[cfg(not(feature = "storage"))]
        unimplemented!("requires 'storage' feature")
    }

    /// Returns the trusted sources.
    pub async fn trusted_sources(&mut self) -> Result<Vec<String>, Error> {
        #[cfg(feature = "storage")]
        {
            self.storage
                .source_preference()
                .fetch_trusted()
                .await
                .map(|set| set.into_iter().collect())
                .map_err(Into::into)
        }

        #[cfg(not(feature = "storage"))]
        unimplemented!("requires 'storage' feature")
    }

    /// Returns the excluded sources.
    pub async fn excluded_sources(&mut self) -> Result<Vec<String>, Error> {
        #[cfg(feature = "storage")]
        {
            self.storage
                .source_preference()
                .fetch_excluded()
                .await
                .map(|set| set.into_iter().collect())
                .map_err(Into::into)
        }

        #[cfg(not(feature = "storage"))]
        unimplemented!("requires 'storage' feature")
    }

    /// Adds a trusted source.
    #[cfg_attr(not(feature = "storage"), allow(unused_variables))]
    pub async fn add_trusted_source(
        &mut self,
        sources: &[WeightedSource],
        new_trusted: String,
    ) -> Result<(), Error> {
        #[cfg(feature = "storage")]
        {
            let mut trusted = self.storage.source_preference().fetch_trusted().await?;
            if !trusted.insert(new_trusted) {
                return Ok(());
            }

            let old_excluded = self.storage.source_preference().fetch_excluded().await?;
            self.storage
                .source_preference()
                .set_trusted(&trusted)
                .await?;

            if trusted.intersection(&old_excluded).next().is_some() {
                // update the endpoint configuration's excluded sources,
                // as the new_trusted source was previously an excluded source
                let updated_excluded: HashSet<String> =
                    old_excluded.difference(&trusted).cloned().collect();
                *self.endpoint_config.excluded_sources.write().await =
                    updated_excluded.iter().cloned().collect();
                self.filter_excluded_sources_for_all_stacks(&updated_excluded)
                    .await;
            }

            *self.endpoint_config.trusted_sources.write().await = trusted.iter().cloned().collect();
            let history = self.storage.fetch_history().await?;
            self.update_stacks_for_all_markets(&history, sources, self.core_config.request_new)
                .await
        }

        #[cfg(not(feature = "storage"))]
        unimplemented!("requires 'storage' feature")
    }

    /// Removes a trusted source.
    #[cfg_attr(not(feature = "storage"), allow(unused_variables))]
    pub async fn remove_trusted_source(
        &mut self,
        sources: &[WeightedSource],
        trusted: String,
    ) -> Result<(), Error> {
        #[cfg(feature = "storage")]
        {
            let mut trusted_set = self.storage.source_preference().fetch_trusted().await?;
            if !trusted_set.remove(&trusted) {
                return Ok(());
            }

            self.storage
                .source_preference()
                .set_trusted(&trusted_set)
                .await?;

            *self.endpoint_config.trusted_sources.write().await =
                trusted_set.iter().cloned().collect();

            let history = self.storage.fetch_history().await?;
            self.update_stacks_for_all_markets(&history, sources, self.core_config.request_new)
                .await
        }

        #[cfg(not(feature = "storage"))]
        unimplemented!("requires 'storage' feature")
    }

    /// Adds an excluded source.
    #[cfg_attr(not(feature = "storage"), allow(unused_variables))]
    pub async fn add_excluded_source(
        &mut self,
        sources: &[WeightedSource],
        new_excluded: String,
    ) -> Result<(), Error> {
        #[cfg(feature = "storage")]
        {
            let mut excluded = self.storage.source_preference().fetch_excluded().await?;
            if !excluded.insert(new_excluded) {
                return Ok(());
            }

            let old_trusted = self.storage.source_preference().fetch_trusted().await?;
            self.storage
                .source_preference()
                .set_excluded(&excluded)
                .await?;

            if excluded.intersection(&old_trusted).next().is_some() {
                // update the endpoint configuration's trusted sources,
                // as the new_excluded source contains was previously a trusted source
                let updated_trusted = old_trusted.difference(&excluded).cloned().collect();
                *self.endpoint_config.trusted_sources.write().await = updated_trusted;
            }

            *self.endpoint_config.excluded_sources.write().await =
                excluded.iter().cloned().collect();
            self.filter_excluded_sources_for_all_stacks(&excluded).await;

            let history = self.storage.fetch_history().await?;
            self.update_stacks_for_all_markets(&history, sources, self.core_config.request_new)
                .await
        }

        #[cfg(not(feature = "storage"))]
        unimplemented!("requires 'storage' feature")
    }

    /// Removes an excluded source.
    #[cfg_attr(not(feature = "storage"), allow(unused_variables))]
    pub async fn remove_excluded_source(
        &mut self,
        sources: &[WeightedSource],
        excluded: String,
    ) -> Result<(), Error> {
        #[cfg(feature = "storage")]
        {
            let mut excluded_set = self.storage.source_preference().fetch_excluded().await?;
            if !excluded_set.remove(&excluded) {
                return Ok(());
            }

            self.storage
                .source_preference()
                .set_excluded(&excluded_set)
                .await?;

            *self.endpoint_config.excluded_sources.write().await =
                excluded_set.iter().cloned().collect();

            let history = self.storage.fetch_history().await?;
            self.update_stacks_for_all_markets(&history, sources, self.core_config.request_new)
                .await
        }

        #[cfg(not(feature = "storage"))]
        unimplemented!("requires 'storage' feature")
    }

    /// Resets the AI state
    pub async fn reset_ai(&mut self) -> Result<(), Error> {
        self.clear_stack_data().await;
        self.exploration_stack =
            Exploration::new(StackData::default(), ExplorationConfig::default())
                .map_err(Error::InvalidStack)?;
        self.state.reset();

        self.request_after = 0;
        self.update_stacks_for_all_markets(&[], &[], usize::MAX)
            .await
            .ok();

        Ok(())
    }
}

/// The ranker could rank the documents in a different order so we update the stacks with it.
fn rank_stacks<'a>(
    stacks: impl Iterator<Item = &'a mut Stack>,
    exploration_stack: &mut Exploration,
    coi: &CoiSystem,
    user_interests: &UserInterests,
) {
    for stack in stacks {
        stack.rank(coi, user_interests);
    }
    exploration_stack.rank(coi, user_interests);
}

/// Updates the stacks with data related to the top key phrases of the current data.
#[allow(clippy::too_many_arguments)]
#[instrument(skip(stacks, exploration_stack, smbert, coi, state, history))]
async fn update_stacks<'a>(
    stacks: &mut HashMap<Id, Stack>,
    exploration_stack: &mut Exploration,
    smbert: &SMBert,
    coi: &CoiSystem,
    state: &mut CoiSystemState,
    history: &[HistoricDocument],
    sources: &[WeightedSource],
    take_top: usize,
    keep_top: usize,
    request_new: usize,
    markets: &[Market],
) -> Result<(), Error> {
    let mut ready_stacks = stacks.len();
    let mut errors = Vec::new();
    let mut all_documents = Vec::new();

    // Needy stacks are the ones for which we want to fetch new items.
    let needy_stacks = stacks
        .values_mut()
        .filter(|stack| stack.len() <= request_new)
        .collect_vec();

    // return early if there are no stacks to be updated
    if needy_stacks.is_empty() {
        info!(message = "no stacks needed an update");
        return Ok(());
    }

    let key_phrases_by_market = needy_stacks
        .iter()
        .any(|stack| stack.ops.needs_key_phrases())
        .then(|| {
            markets
                .iter()
                .map(|market| {
                    let key_phrases = coi.take_key_phrases(
                        &state.user_interests.positive,
                        market,
                        &mut state.key_phrases,
                        take_top,
                    );
                    (market, key_phrases)
                })
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();

    // Here we gather new documents for all relevant stacks, and put them into a vector.
    // We don't update the stacks immediately, because we want to de-duplicate the documents
    // across stacks first.
    let new_document_futures = needy_stacks
        .iter()
        .flat_map(|stack| {
            markets.iter().map(|market| {
                let key_phrases = key_phrases_by_market
                    .get(market)
                    .map_or(&[] as &[_], Vec::as_slice);
                fetch_new_documents_for_stack(stack, smbert, key_phrases, history, market)
            })
        })
        .collect_vec();

    let mut stacks_not_ready = HashSet::<StackId>::new();
    for maybe_new_documents in join_all(new_document_futures).await {
        match maybe_new_documents {
            Err(Error::StackOpFailed(stack::Error::New(NewItemsError::NotReady(stack_id)))) => {
                stacks_not_ready.insert(stack_id);
                continue;
            }
            Err(error) => {
                error!("{}", error);
                errors.push(error);
                continue;
            }
            Ok(documents) => all_documents.extend(documents),
        };
    }
    ready_stacks -= stacks_not_ready.len();

    // Since we need to de-duplicate not only the newly retrieved documents among themselves,
    // but also consider the existing documents in all stacks (not just the needy ones), we extract
    // them into `all_documents` here.
    for stack in stacks.values_mut() {
        all_documents.extend(stack.drain_documents());
    }
    all_documents.extend(exploration_stack.drain_documents());

    // Separate stack documents (via the `stack` parameter) are not needed here, since they are
    // already contained in `all_documents`.
    all_documents = DuplicateFilter::apply(history, &[], all_documents);
    let max_clusters = all_documents.len() / 2;
    all_documents = filter_semantically(
        all_documents,
        sources,
        &SemanticFilterConfig {
            criterion: Criterion::MaxClusters(max_clusters),
            ..SemanticFilterConfig::default()
        },
    );

    // Filter the exploration stack documents from the other documents, in order
    // to keep the loop below simple.
    let (mut exploration_docs, other_docs): (Vec<Document>, Vec<Document>) = all_documents
        .into_iter()
        .partition(|doc| doc.stack_id == Exploration::id());

    // Finally, we can update the stacks with their respective documents. To do this, we
    // have to group the fetched documents by `stack_id`, then `update` the stacks.
    let documents_by_stack_id = other_docs.into_iter().into_group_map_by(|doc| doc.stack_id);

    for (stack_id, documents_group) in documents_by_stack_id {
        // .unwrap() is safe here, because each document is created by an existing stack,
        // the `stacks` HashMap contains all instantiated stacks, and the documents that belong
        // to the exploration stack have been filtered before.
        let stack = stacks.get_mut(&stack_id).unwrap();

        if let Err(error) = stack.update(&documents_group, coi, &state.user_interests) {
            let error = Error::StackOpFailed(error);
            error!("{}", error);
            errors.push(error);
        } else {
            let is_breaking_news = stack.id() == BreakingNews::id();
            if let (true, Some(documents)) = (is_breaking_news, stack.data.retain_top(keep_top)) {
                exploration_docs.extend(documents);
            }
        }
    }

    if let Err(error) = exploration_stack.update(&exploration_docs, coi, &state.user_interests) {
        let error = Error::StackOpFailed(error);
        error!("{}", error);
        errors.push(error);
    } else {
        exploration_stack.data.retain_top(keep_top);
    }

    if tracing::enabled!(Level::DEBUG) {
        for (id, data) in stacks
            .values()
            .map(|stack| (stack.id(), &stack.data))
            .chain(once((Exploration::id(), &exploration_stack.data)))
        {
            for (ranking, document) in data.documents.iter().rev().enumerate() {
                debug!(
                    stack = %id,
                    document = %document.id,
                    stack_ranking = ranking,
                    title = %document.resource.title
                );
            }
        }
    }

    // only return an error if all stacks that were ready to get new items failed
    if !errors.is_empty() && errors.len() >= ready_stacks {
        Err(Error::Errors(errors))
    } else {
        Ok(())
    }
}

async fn fetch_new_documents_for_stack(
    stack: &Stack,
    smbert: &SMBert,
    key_phrases: &[KeyPhrase],
    history: &[HistoricDocument],
    market: &Market,
) -> Result<Vec<Document>, Error> {
    let articles = match stack.new_items(key_phrases, history, market).await {
        Ok(articles) => articles,
        Err(error) => {
            return Err(Error::StackOpFailed(error));
        }
    };
    let (documents, errors) = documentify_articles(stack.id(), smbert, articles);

    // only return an error if all articles failed
    if documents.is_empty() && !errors.is_empty() {
        Err(Error::Errors(errors))
    } else {
        Ok(documents)
    }
}

fn documentify_articles(
    stack_id: StackId,
    smbert: &SMBert,
    articles: Vec<GenericArticle>,
) -> (Vec<Document>, Vec<Error>) {
    articles
        .into_par_iter()
        .map(|article| {
            let embedding = match &article.embedding {
                Some(embedding) if embedding.len() == SMBert::embedding_size() => {
                    Embedding::from(Array::from_vec(embedding.clone()))
                }
                Some(_) | None => smbert.run(article.snippet_or_title()).map_err(|error| {
                    let error = Error::Ranker(error.into());
                    error!("{}", error);
                    error
                })?,
            };
            (article, stack_id, embedding).try_into().map_err(|error| {
                let error = Error::Document(error);
                error!("{}", error);
                error
            })
        })
        .partition_map(|result| match result {
            Ok(document) => Either::Left(document),
            Err(error) => Either::Right(error),
        })
}

fn documentify_topics(smbert: &SMBert, topics: Vec<BingTopic>) -> (Vec<TrendingTopic>, Vec<Error>) {
    topics
        .into_par_iter()
        .map(|topic| {
            let embedding = smbert.run(&topic.name).map_err(|error| {
                let error = Error::Ranker(error.into());
                error!("{}", error);
                error
            })?;
            (topic, embedding).try_into().map_err(|error| {
                let error = Error::Document(error);
                error!("{}", error);
                error
            })
        })
        .partition_map(|result| match result {
            Ok(topic) => Either::Left(topic),
            Err(error) => Either::Right(error),
        })
}

#[derive(Serialize, Deserialize)]
struct SerializedStackState(Vec<u8>);

impl SerializedStackState {
    fn deserialize(&self) -> Result<HashMap<StackId, StackData>, Error> {
        bincode::deserialize(&self.0).map_err(Error::Deserialization)
    }
}

#[derive(Serialize, Deserialize)]
struct SerializedCoiSystemState(Vec<u8>);

impl SerializedCoiSystemState {
    fn deserialize(&self) -> Result<CoiSystemState, Error> {
        CoiSystemState::deserialize(&self.0).map_err(Into::into)
    }
}

#[derive(Serialize, Deserialize)]
struct SerializedState {
    /// The serialized stacks state.
    stacks: SerializedStackState,
    /// The serialized coi system state.
    coi: SerializedCoiSystemState,
}

impl SerializedState {
    fn deserialize(state: &[u8]) -> Result<(HashMap<StackId, StackData>, CoiSystemState), Error> {
        let state = bincode::deserialize::<Self>(state).map_err(Error::Deserialization)?;
        let stacks = state.stacks.deserialize()?;
        let coi = state.coi.deserialize()?;

        Ok((stacks, coi))
    }
}

/// Active search mode.
pub enum SearchBy<'a> {
    /// Search by query.
    Query(Cow<'a, Filter>),
    /// Search by topic.
    Topic(Cow<'a, str>),
}

#[cfg(test)]
pub(crate) mod tests {
    use std::mem::size_of;

    use async_once_cell::OnceCell;
    use chrono::NaiveDate;
    use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};
    use url::Url;
    use wiremock::{
        matchers::{method, path},
        Mock,
        MockServer,
        ResponseTemplate,
    };

    use crate::{document::tests::mock_generic_article, stack::ops::MockOps};
    use xayn_discovery_engine_providers::{Rank, UrlWithDomain};
    use xayn_discovery_engine_test_utils::smbert::{model, vocab};

    use super::*;

    #[test]
    fn test_usize_not_to_small() {
        assert!(size_of::<usize>() >= size_of::<u32>());
    }

    /// A shared test engine.
    ///
    /// The systems are only initialized once to save time and the states can be configured for each
    /// test via [`init_engine()`].
    static ENGINE: OnceCell<Mutex<(MockServer, Engine)>> = OnceCell::new();

    /// Initializes the states of the test [`ENGINE`].
    ///
    /// The stacks can be configured with specific stack ops. The stack states are either blank or
    /// can optionally be updated once like in a freshly started engine.
    async fn init_engine<'a, I, F>(
        stack_ops: I,
        update_after_reset: bool,
    ) -> MappedMutexGuard<'a, Engine>
    where
        I: IntoIterator<Item = F> + Send,
        <I as IntoIterator>::IntoIter: Send,
        F: FnOnce(&EndpointConfig, &Providers) -> BoxedOps,
    {
        let init_engine = async move {
            // We need a mock server from which the initialized stacks can fetch articles
            let server = MockServer::start().await;
            let tmpl = ResponseTemplate::new(200)
                .set_body_string(include_str!("../test-fixtures/newscatcher/duplicates.json"));

            Mock::given(method("POST"))
                .and(path("/newscatcher/headlines-endpoint-name"))
                .respond_with(tmpl)
                .mount(&server)
                .await;

            // The config mostly tells the engine were to find the model assets.
            // Here we use the mocked ones, for speed.
            let asset_base = "../../discovery_engine_flutter/example/assets";
            let config = InitConfig {
                api_key: "test-token".into(),
                api_base_url: server.uri(),
                news_provider_path: "newscatcher/news-endpoint-name".into(),
                headlines_provider_path: "newscatcher/headlines-endpoint-name".into(),
                markets: vec![Market::new("en", "US")],
                // This triggers the trusted sources stack to also fetch articles
                trusted_sources: vec!["example.com".into()],
                excluded_sources: vec![],
                smbert_vocab: format!("{asset_base}/smbert_v0001/vocab.txt"),
                smbert_model: format!("{asset_base}/smbert_v0001/smbert-mocked.onnx"),
                kpe_vocab: format!("{asset_base}/kpe_v0001/vocab.txt"),
                kpe_model: format!("{asset_base}/kpe_v0001/bert-mocked.onnx"),
                kpe_cnn: format!("{asset_base}/kpe_v0001/cnn.binparams"),
                kpe_classifier: format!("{asset_base}/kpe_v0001/classifier.binparams"),
                max_docs_per_feed_batch: FeedConfig::default()
                    .max_docs_per_batch
                    .try_into()
                    .unwrap_or(u32::MAX),
                max_docs_per_search_batch: SearchConfig::default()
                    .max_docs_per_batch
                    .try_into()
                    .unwrap_or(u32::MAX),
                de_config: None,
                log_file: None,
                data_dir: "tmp_test_data_dir".into(),
                use_in_memory_db: true,
            };

            // Now we can initialize the engine with no previous history or state. This should
            // be the same as when it's initialized for the first time after the app is downloaded.
            let engine = Engine::from_config(config, None, &[], &[]).await.unwrap();

            Mutex::new((server, engine))
        };
        let mut engine = MutexGuard::map(
            ENGINE.get_or_init(init_engine).await.lock().await,
            |engine| &mut engine.1,
        );

        // reset the stacks and states
        #[cfg(feature = "storage")]
        {
            engine.storage = {
                let storage = SqliteStorage::connect("sqlite::memory:").await.unwrap();
                storage.init_database().await.unwrap();
                Box::new(storage) as BoxedStorage
            };
        }
        engine.stacks = RwLock::new(
            stack_ops
                .into_iter()
                .map(|stack_ops_new| {
                    let stack = Stack::new(
                        StackData::default(),
                        stack_ops_new(&engine.endpoint_config, &engine.providers),
                        StackConfig::default(),
                    )
                    .unwrap();
                    (stack.id(), stack)
                })
                .collect(),
        );
        engine.exploration_stack.data = StackData::default();
        engine.state = CoiSystemState::default();

        if update_after_reset {
            engine
                .update_stacks_for_all_markets(&[], &[], usize::MAX)
                .await
                .unwrap();
        }

        engine
    }

    fn new_mock_stack_ops() -> MockOps {
        let stack_id = Id::new_random();
        let mut mock_ops = MockOps::new();
        mock_ops.expect_id().returning(move || stack_id);
        mock_ops.expect_needs_key_phrases().returning(|| true);
        mock_ops
            .expect_merge()
            .returning(|stack, new| Ok(chain!(stack, new).cloned().collect()));
        mock_ops
    }

    #[tokio::test]
    async fn test_cross_stack_deduplication() {
        // We assume that, if de-duplication works between two stacks, it'll work between
        // any number of stacks. So we just create two.
        let engine = &mut *init_engine(
            [
                |config: &EndpointConfig, providers: &Providers| {
                    Box::new(BreakingNews::new(config, providers.headlines.clone())) as _
                },
                |config: &EndpointConfig, providers: &Providers| {
                    Box::new(TrustedNews::new(
                        config,
                        providers.trusted_headlines.clone(),
                    )) as _
                },
            ],
            false,
        )
        .await;

        // Stacks should be empty before we start fetching anything
        let mut stacks = engine.stacks.write().await;
        for stack in stacks.values() {
            assert!(stack.is_empty());
        }

        // Update stacks does a lot of things, what's relevant for us is that
        //      a) it fetches new documents
        //      b) it's supposed to de-duplicate between stacks
        // in that order.
        update_stacks(
            &mut stacks,
            &mut engine.exploration_stack,
            &engine.smbert,
            &engine.coi,
            &mut engine.state,
            &[],
            &[],
            10,
            10,
            10,
            &[Market::new("en", "US")],
        )
        .await
        .unwrap();

        // After calling `update_stacks` once, one of the two stacks should contain one document.
        // Both stacks fetched the same item, but de-duplication should prevent the same document
        // being added to both stacks.
        assert_eq!(stacks.values().map(Stack::len).sum::<usize>(), 1);

        // Now we call `update_stacks` again. We do this to ensure that de-duplication also takes
        // into account the items that are already present inside the stacks, and not only the
        // newly fetched documents.
        update_stacks(
            &mut stacks,
            &mut engine.exploration_stack,
            &engine.smbert,
            &engine.coi,
            &mut engine.state,
            &[],
            &[],
            10,
            10,
            10,
            &[Market::new("en", "US")],
        )
        .await
        .unwrap();

        // No new documents should have been added by the second `update_stacks` call.
        assert_eq!(stacks.values().map(Stack::len).sum::<usize>(), 1);
    }

    #[tokio::test]
    async fn test_update_stack_no_error_when_no_stack_is_ready() {
        let engine = &mut *init_engine(
            [|_: &'_ _, _: &'_ _| {
                let mut mock_ops = new_mock_stack_ops();
                let id = mock_ops.id();
                mock_ops
                    .expect_new_items()
                    .returning(move |_, _, _, _| Err(NewItemsError::NotReady(id)));
                Box::new(mock_ops) as _
            }],
            false,
        )
        .await;

        update_stacks(
            &mut *engine.stacks.write().await,
            &mut engine.exploration_stack,
            &engine.smbert,
            &engine.coi,
            &mut engine.state,
            &[],
            &[],
            10,
            10,
            10,
            &[Market::new("en", "US")],
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_update_stack_multiple_market_stack_not_ready() {
        // When handling the errors in update_stacks we were counting
        // a not ready stack for each market in the configuration.

        let engine = &mut *init_engine(
            [|_: &'_ _, _: &'_ _| {
                let mut mock_ops = new_mock_stack_ops();
                let id = mock_ops.id();
                mock_ops
                    .expect_new_items()
                    .returning(move |_, _, _, _| Err(NewItemsError::NotReady(id)));
                Box::new(mock_ops) as _
            }],
            false,
        )
        .await;

        engine
            .set_markets(
                &[],
                &[],
                vec![Market::new("de", "DE"), Market::new("it", "IT")],
            )
            .await
            .unwrap();

        update_stacks(
            &mut *engine.stacks.write().await,
            &mut engine.exploration_stack,
            &engine.smbert,
            &engine.coi,
            &mut engine.state,
            &[],
            &[],
            10,
            10,
            10,
            &[Market::new("en", "US")],
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_update_stack_no_error_when_one_stack_is_successful() {
        let engine = &mut *init_engine(
            [
                |_: &'_ _, _: &'_ _| {
                    let mut mock_ops_ok = new_mock_stack_ops();
                    mock_ops_ok
                        .expect_new_items()
                        .returning(|_, _, _, _| Ok(vec![mock_generic_article()]));
                    Box::new(mock_ops_ok) as _
                },
                |_: &'_ _, _: &'_ _| {
                    let mut mock_ops_failed = new_mock_stack_ops();
                    mock_ops_failed.expect_new_items().returning(|_, _, _, _| {
                        Err(NewItemsError::Error("mock_ops_failed_error".into()))
                    });
                    Box::new(mock_ops_failed) as _
                },
            ],
            false,
        )
        .await;

        update_stacks(
            &mut *engine.stacks.write().await,
            &mut engine.exploration_stack,
            &engine.smbert,
            &engine.coi,
            &mut engine.state,
            &[],
            &[],
            10,
            10,
            10,
            &[Market::new("en", "US")],
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_update_stack_should_error_when_all_stacks_fail() {
        let engine = &mut *init_engine(
            [|_: &'_ _, _: &'_ _| {
                let mut mock_ops_failed = new_mock_stack_ops();
                mock_ops_failed.expect_new_items().returning(|_, _, _, _| {
                    Err(NewItemsError::Error("mock_ops_failed_error".into()))
                });
                Box::new(mock_ops_failed) as _
            }],
            false,
        )
        .await;

        let result = update_stacks(
            &mut *engine.stacks.write().await,
            &mut engine.exploration_stack,
            &engine.smbert,
            &engine.coi,
            &mut engine.state,
            &[],
            &[],
            10,
            10,
            10,
            &[Market::new("en", "US")],
        )
        .await;

        if let Err(Error::Errors(errors)) = result {
            match &errors.as_slice() {
                &[Error::StackOpFailed(stack::Error::New(NewItemsError::Error(msg)))] => {
                    assert_eq!(msg.to_string(), "mock_ops_failed_error".to_string());
                }
                x => panic!("Wrong result returned: {:?}", x),
            }
        } else {
            panic!("Wrong error structure");
        }
    }

    #[tokio::test]
    async fn test_basic_engine_integration() {
        let engine = &mut *init_engine(
            [
                |config: &EndpointConfig, providers: &Providers| {
                    Box::new(BreakingNews::new(config, providers.headlines.clone())) as _
                },
                |config: &EndpointConfig, providers: &Providers| {
                    Box::new(TrustedNews::new(
                        config,
                        providers.trusted_headlines.clone(),
                    )) as _
                },
                |config: &EndpointConfig, providers: &Providers| {
                    Box::new(PersonalizedNews::new(
                        config,
                        providers.similar_news.clone(),
                    )) as _
                },
            ],
            true,
        )
        .await;

        // Finally, we instruct the engine to fetch some articles and check whether or not
        // the expected articles from the mock show up in the results.
        let res = engine.get_feed_documents(&[], &[]).await.unwrap();

        assert_eq!(1, res.len());
        assert_eq!(
            res.get(0).unwrap().resource.title,
            "Some really important article",
        );
    }

    fn example_url() -> UrlWithDomain {
        let url = Url::parse("https://example.net").unwrap(/* used only in tests */);
        UrlWithDomain::new(url).unwrap(/* used only in tests */)
    }

    #[test]
    fn test_documentify_articles() {
        let smbert = SMBertConfig::from_files(vocab().unwrap(), model().unwrap())
            .unwrap()
            .with_pooling::<AveragePooler>()
            .build()
            .unwrap();
        let stack_id = StackId::new_random();
        let size = SMBert::embedding_size();
        let embedding_1 = vec![1.; size];
        let embedding_2 = vec![2.; size + 1];
        let article_1 = GenericArticle {
            title: String::default(),
            snippet: String::default(),
            url: example_url(),
            image: None,
            date_published: NaiveDate::from_ymd(2022, 1, 1).and_hms(9, 0, 0),
            score: None,
            rank: Rank::default(),
            country: "US".to_string(),
            language: "en".to_string(),
            topic: "news".to_string(),
            embedding: Some(embedding_1.clone()),
        };
        let article_2 = GenericArticle {
            embedding: Some(embedding_2.clone()),
            ..article_1.clone()
        };

        let expected_1 = Embedding::from(Array::from_vec(embedding_1));
        let expected_2 = Embedding::from(Array::from_vec(embedding_2));
        let (documents, _) = documentify_articles(stack_id, &smbert, vec![article_1, article_2]);

        assert_eq!(documents[0].smbert_embedding, expected_1);
        assert_ne!(documents[1].smbert_embedding, expected_2);
    }
}
