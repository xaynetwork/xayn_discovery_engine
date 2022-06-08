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

use std::{
    collections::{HashMap, HashSet},
    iter::once,
    mem::replace,
    sync::Arc,
};

use displaydoc::Display;
use figment::{
    providers::{Format, Json, Serialized},
    Figment,
};
use futures::future::join_all;
use itertools::{chain, Itertools};
use rayon::iter::{Either, IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::error;

use xayn_discovery_engine_ai::{
    cosine_similarity,
    nan_safe_f32_cmp,
    Builder,
    CoiSystemConfig,
    Embedding,
    GenericError,
    KeyPhrase,
};
use xayn_discovery_engine_bert::{AveragePooler, SMBertConfig};
use xayn_discovery_engine_kpe::Config as KpeConfig;
use xayn_discovery_engine_providers::{
    clean_query,
    Article,
    Client,
    CommonQueryParts,
    Filter,
    HeadlinesQuery,
    Market,
    NewsQuery,
    TrendingQuery,
    TrendingTopic as BingTopic,
};
use xayn_discovery_engine_tokenizer::{AccentChars, CaseChars};

use crate::{
    document::{
        self,
        Document,
        HistoricDocument,
        TimeSpent,
        TrendingTopic,
        UserReacted,
        UserReaction,
    },
    mab::{self, BetaSampler, Bucket, SelectionIter},
    ranker::Ranker,
    stack::{
        self,
        exploration,
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

    /// List of errors/warnings. {0:?}
    Errors(Vec<Error>),
}

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

/// Temporary config to allow for configurations within the core without a mirroring outside impl.
struct CoreConfig {
    /// The number of taken top key phrases while updating the stacks.
    take_top: usize,
    /// The number of top documents per stack to keep while filtering the stacks.
    keep_top: usize,
    /// The lower bound of documents per stack at which new items are requested.
    request_new: usize,
    /// The number of times to get feed documents after which the stacks are updated without the
    /// limitation of `request_new`.
    request_after: usize,
    /// The maximum number of top key phrases extracted from the search term in the deep search.
    deep_search_top: usize,
    /// The maximum number of documents returned from the deep search.
    deep_search_max: usize,
    /// The minimum cosine similarity wrt the original document below which documents returned from
    /// the deep search are discarded.
    deep_search_sim: f32,
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

/// Discovery Engine.
pub struct Engine<R> {
    client: Arc<Client>,
    config: EndpointConfig,
    core_config: CoreConfig,
    stacks: RwLock<HashMap<StackId, Stack>>,
    exploration_stack: exploration::Stack,
    ranker: R,
    request_after: usize,
}

impl<R> Engine<R>
where
    R: Ranker + Send + Sync,
{
    /// Creates a new `Engine`.
    async fn new(
        config: EndpointConfig,
        ranker: R,
        history: &[HistoricDocument],
        stack_ops: Vec<BoxedOps>,
        client: Arc<Client>,
    ) -> Result<Self, Error> {
        let stack_data = |_| StackData::default();

        Self::from_stack_data(
            config,
            ranker,
            history,
            stack_data,
            StackData::default(),
            stack_ops,
            client,
        )
        .await
    }

    /// Creates a new `Engine` from serialized state and stack operations.
    ///
    /// The `Engine` only keeps in its state data related to the current [`BoxedOps`].
    /// Data related to missing operations will be dropped.
    async fn from_state<'a>(
        state: &'a StackState,
        config: EndpointConfig,
        ranker: R,
        history: &'a [HistoricDocument],
        stack_ops: Vec<BoxedOps>,
        client: Arc<Client>,
    ) -> Result<Self, Error> {
        if stack_ops.is_empty() {
            return Err(Error::NoStackOps);
        }

        let mut stack_data = bincode::deserialize::<HashMap<StackId, _>>(&state.0)
            .map_err(Error::Deserialization)?;
        let exploration_stack_data = stack_data
            .remove(&exploration::Stack::id())
            .unwrap_or_default();
        let stack_data = |id| stack_data.remove(&id).unwrap_or_default();

        Self::from_stack_data(
            config,
            ranker,
            history,
            stack_data,
            exploration_stack_data,
            stack_ops,
            client,
        )
        .await
    }

    async fn from_stack_data(
        config: EndpointConfig,
        ranker: R,
        history: &[HistoricDocument],
        mut stack_data: impl FnMut(StackId) -> StackData + Send,
        exploration_stack_data: StackData,
        stack_ops: Vec<BoxedOps>,
        client: Arc<Client>,
    ) -> Result<Self, Error> {
        let stacks = stack_ops
            .into_iter()
            .map(|ops| {
                let id = ops.id();
                let data = stack_data(id);
                Stack::new(data, ops).map(|stack| (id, stack))
            })
            .collect::<Result<HashMap<_, _>, _>>()
            .map_err(Error::InvalidStack)?;
        let core_config = CoreConfig::default();

        let exploration_stack =
            exploration::Stack::new(exploration_stack_data).map_err(Error::InvalidStack)?;

        // we don't want to fail initialization if there are network problems
        let mut engine = Self {
            client,
            config,
            core_config,
            stacks: RwLock::new(stacks),
            exploration_stack,
            ranker,
            request_after: 0,
        };

        engine
            .update_stacks_for_all_markets(history, usize::MAX)
            .await
            .ok();

        Ok(engine)
    }

    async fn update_stacks_for_all_markets(
        &mut self,
        history: &[HistoricDocument],
        request_new: usize,
    ) -> Result<(), Error> {
        let markets = self.config.markets.read().await;
        let mut stacks = self.stacks.write().await;

        update_stacks(
            &mut stacks,
            &mut self.exploration_stack,
            &mut self.ranker,
            history,
            self.core_config.take_top,
            self.core_config.keep_top,
            request_new,
            &markets,
        )
        .await
    }

    /// Serializes the state of the `Engine` and `Ranker` state.
    pub async fn serialize(&self) -> Result<Vec<u8>, Error> {
        let stacks = self.stacks.read().await;
        let mut stacks_data = stacks
            .iter()
            .map(|(id, stack)| (id, &stack.data))
            .collect::<HashMap<_, _>>();
        let exploration_stack_id = exploration::Stack::id();
        stacks_data.insert(&exploration_stack_id, &self.exploration_stack.data);

        let engine = bincode::serialize(&stacks_data)
            .map(StackState)
            .map_err(|err| Error::Serialization(err.into()))?;

        let ranker = self
            .ranker
            .serialize()
            .map(RankerState)
            .map_err(Error::Serialization)?;

        let state_data = State { engine, ranker };

        bincode::serialize(&state_data).map_err(|err| Error::Serialization(err.into()))
    }

    /// Updates the markets configuration.
    ///
    /// Also resets and updates all stacks.
    pub async fn set_markets(
        &mut self,
        history: &[HistoricDocument],
        new_markets: Vec<Market>,
    ) -> Result<(), Error> {
        let mut markets_guard = self.config.markets.write().await;
        let mut old_markets = replace(&mut *markets_guard, new_markets);
        old_markets.retain(|market| !markets_guard.contains(market));
        self.ranker.remove_key_phrases(&old_markets);
        drop(markets_guard);

        self.clear_stack_data().await;

        self.update_stacks_for_all_markets(history, self.core_config.request_new)
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

    /// Returns at most `max_documents` [`Document`]s for the feed.
    pub async fn get_feed_documents(
        &mut self,
        history: &[HistoricDocument],
        max_documents: u32,
    ) -> Result<Vec<Document>, Error> {
        let request_new = (self.request_after < self.core_config.request_after)
            .then(|| self.core_config.request_new)
            .unwrap_or(usize::MAX);

        self.update_stacks_for_all_markets(history, request_new)
            .await?;

        self.request_after = (self.request_after + 1) % self.core_config.request_after;

        let mut stacks = self.stacks.write().await;
        let mut all_stacks = chain!(
            stacks.values_mut().map(|s| s as _),
            once(&mut self.exploration_stack as _),
        )
        .collect::<Vec<&mut dyn Bucket<_>>>();

        SelectionIter::new(BetaSampler, all_stacks.iter_mut())
            .select(max_documents as usize)
            .map_err(Into::into)
    }

    /// Process the feedback about the user spending some time on a document.
    pub async fn time_spent(&mut self, time_spent: &TimeSpent) -> Result<(), Error> {
        self.ranker.log_document_view_time(time_spent)?;

        rank_stacks(
            self.stacks.write().await.values_mut(),
            &mut self.exploration_stack,
            &mut self.ranker,
        )
    }

    /// Process the feedback about the user reacting to a document.
    ///
    /// The history is only required for positive reactions.
    pub async fn user_reacted(
        &mut self,
        history: Option<&[HistoricDocument]>,
        reacted: &UserReacted,
    ) -> Result<(), Error> {
        let mut stacks = self.stacks.write().await;

        // update relevance of stack if the reacted document belongs to one
        if !reacted.stack_id.is_nil() {
            if let Some(stack) = stacks.get_mut(&reacted.stack_id) {
                stack.update_relevance(reacted.reaction);
            } else if reacted.stack_id == exploration::Stack::id() {
                self.exploration_stack.update_relevance(reacted.reaction);
            } else {
                return Err(Error::InvalidStackId(reacted.stack_id));
            }
        };

        self.ranker.log_user_reaction(reacted)?;

        rank_stacks(
            stacks.values_mut(),
            &mut self.exploration_stack,
            &mut self.ranker,
        )?;
        if let UserReaction::Positive = reacted.reaction {
            if let Some(history) = history {
                update_stacks(
                    &mut stacks,
                    &mut self.exploration_stack,
                    &mut self.ranker,
                    history,
                    self.core_config.take_top,
                    self.core_config.keep_top,
                    usize::MAX,
                    &[reacted.market.clone()],
                )
                .await?;
                self.request_after = 0;
                Ok(())
            } else {
                Err(Error::StackOpFailed(stack::Error::NoHistory))
            }
        } else {
            Ok(())
        }
    }

    /// Perform an active search by query.
    pub async fn search_by_query(
        &mut self,
        query: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Document>, Error> {
        let query = clean_query(query);
        if query.trim().is_empty() {
            return Err(Error::InvalidTerm);
        }
        let filter = &Filter::default().add_keyword(&query);
        self.active_search(SearchBy::Query(filter), page, page_size)
            .await
    }

    /// Perform an active search by topic.
    pub async fn search_by_topic(
        &mut self,
        topic: &str,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Document>, Error> {
        if topic.trim().is_empty() {
            return Err(Error::InvalidTerm);
        }
        self.active_search(SearchBy::Topic(topic), page, page_size)
            .await
    }

    async fn active_search(
        &mut self,
        by: SearchBy<'_>,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<Document>, Error> {
        let mut errors = Vec::new();
        let mut articles = Vec::new();

        let markets = self.config.markets.read().await;
        let scaled_page_size = page_size as usize / markets.len() + 1;
        let excluded_sources = self.config.excluded_sources.read().await.clone();
        for market in markets.iter() {
            let common = CommonQueryParts {
                market: Some(market),
                page_size: scaled_page_size,
                page: page as usize,
                excluded_sources: &excluded_sources,
            };
            let query_result = match by {
                SearchBy::Query(filter) => {
                    let news_query = NewsQuery {
                        common,
                        filter,
                        max_age_days: None,
                    };
                    self.client.query_articles(&news_query).await
                }
                SearchBy::Topic(topic) => {
                    let headlines_query = HeadlinesQuery {
                        common,
                        trusted_sources: &[],
                        topic: Some(topic),
                        max_age_days: None,
                    };
                    self.client.query_articles(&headlines_query).await
                }
            };
            query_result.map_or_else(
                |err| errors.push(Error::Client(err.into())),
                |batch| articles.extend(batch),
            );
        }

        let (mut documents, article_errors) = documentify_articles(
            StackId::nil(), // these documents are not associated with a stack
            &self.ranker,
            articles,
        );
        errors.extend(article_errors);

        if let Err(err) = self.ranker.rank(&mut documents) {
            errors.push(Error::Ranker(err));
        };
        if documents.is_empty() && !errors.is_empty() {
            Err(Error::Errors(errors))
        } else {
            documents.truncate(page_size as usize);
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
        let key_phrases = self.ranker.extract_key_phrases(&clean_query(term))?;
        if key_phrases.is_empty() {
            return Ok(Vec::new());
        }

        let excluded_sources = &self.config.excluded_sources.read().await.clone();
        let filter = &key_phrases
            .iter()
            .take(self.core_config.deep_search_top)
            .fold(Filter::default(), |filter, key_phrase| {
                filter.add_keyword(key_phrase)
            });
        let query = NewsQuery {
            common: CommonQueryParts {
                market: Some(market),
                page_size: self.core_config.deep_search_max,
                page: 1,
                excluded_sources,
            },
            filter,
            max_age_days: None,
        };

        let articles = self
            .client
            .query_articles(&query)
            .await
            .map_err(|error| Error::Client(error.into()))?;
        let articles = MalformedFilter::apply(&[], &[], articles)?;
        let (documents, errors) = documentify_articles(
            StackId::nil(), // these documents are not associated with a stack
            &self.ranker,
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

        let markets = self.config.markets.read().await;
        for market in markets.iter() {
            let query = TrendingQuery { market };
            match self.client.query_trending(&query).await {
                Ok(batch) => topics.extend(batch),
                Err(err) => errors.push(Error::Client(err.into())),
            };
        }

        let (mut topics, topic_errors) = documentify_topics(&self.ranker, topics);
        errors.extend(topic_errors);

        if let Err(err) = self.ranker.rank(&mut topics) {
            errors.push(Error::Ranker(err));
        };
        if topics.is_empty() && !errors.is_empty() {
            Err(Error::Errors(errors))
        } else {
            Ok(topics)
        }
    }

    /// Updates the trusted sources.
    pub async fn set_trusted_sources(
        &mut self,
        history: &[HistoricDocument],
        sources: Vec<String>,
    ) -> Result<(), Error> {
        let sources_set = sources.iter().cloned().collect::<HashSet<_>>();
        *self.config.trusted_sources.write().await = sources;

        let mut stacks = self.stacks.write().await;
        for stack in stacks.values_mut() {
            stack.prune_by_sources(&sources_set, false);
        }
        drop(stacks); // guard
        self.exploration_stack.prune_by_sources(&sources_set, false);

        self.update_stacks_for_all_markets(history, self.core_config.request_new)
            .await
    }

    /// Sets a new list of excluded sources
    pub async fn set_excluded_sources(
        &mut self,
        history: &[HistoricDocument],
        excluded_sources: Vec<String>,
    ) -> Result<(), Error> {
        let exclusion_set = excluded_sources.iter().cloned().collect::<HashSet<_>>();
        *self.config.excluded_sources.write().await = excluded_sources;

        let mut stacks = self.stacks.write().await;
        for stack in stacks.values_mut() {
            stack.prune_by_sources(&exclusion_set, true);
        }
        drop(stacks); // guard
        self.exploration_stack
            .prune_by_sources(&exclusion_set, true);

        self.update_stacks_for_all_markets(history, self.core_config.request_new)
            .await
    }

    /// Resets the AI state
    pub async fn reset_ai(&mut self) -> Result<(), Error> {
        self.clear_stack_data().await;
        self.exploration_stack =
            exploration::Stack::new(StackData::default()).map_err(Error::InvalidStack)?;
        self.ranker.reset_ai();

        self.update_stacks_for_all_markets(&[], self.core_config.request_new)
            .await
            .ok();

        Ok(())
    }
}

/// The ranker could rank the documents in a different order so we update the stacks with it.
fn rank_stacks<'a>(
    stacks: impl Iterator<Item = &'a mut Stack>,
    exploration_stack: &mut exploration::Stack,
    ranker: &mut impl Ranker,
) -> Result<(), Error> {
    let mut errors = stacks.fold(Vec::new(), |mut errors, stack| {
        if let Err(error) = stack.rank(ranker) {
            errors.push(Error::StackOpFailed(error));
        }

        errors
    });

    if let Err(error) = exploration_stack.rank(ranker) {
        errors.push(Error::StackOpFailed(error));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::Errors(errors))
    }
}

/// Updates the stacks with data related to the top key phrases of the current data.
#[allow(clippy::too_many_arguments)]
async fn update_stacks<'a>(
    stacks: &mut HashMap<Id, Stack>,
    exploration_stack: &mut exploration::Stack,
    ranker: &mut (impl Ranker + Send + Sync),
    history: &[HistoricDocument],
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
        return Ok(());
    }

    let key_phrases_by_market = needy_stacks
        .iter()
        .any(|stack| stack.ops.needs_key_phrases())
        .then(|| {
            markets
                .iter()
                .map(|market| (market, ranker.take_key_phrases(market, take_top)))
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
                fetch_new_documents_for_stack(stack, ranker, key_phrases, history, market)
            })
        })
        .collect_vec();

    for maybe_new_documents in join_all(new_document_futures).await {
        match maybe_new_documents {
            Err(Error::StackOpFailed(stack::Error::New(NewItemsError::NotReady))) => {
                ready_stacks -= 1;
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
        &SemanticFilterConfig {
            criterion: Criterion::MaxClusters(max_clusters),
            ..SemanticFilterConfig::default()
        },
    );

    // Filter the exploration stack documents from the other documents, in order
    // to keep the loop below simple.
    let (mut exploration_docs, other_docs): (Vec<Document>, Vec<Document>) = all_documents
        .into_iter()
        .partition(|doc| doc.stack_id == exploration::Stack::id());

    // Finally, we can update the stacks with their respective documents. To do this, we
    // have to group the fetched documents by `stack_id`, then `update` the stacks.
    let documents_by_stack_id = other_docs.into_iter().into_group_map_by(|doc| doc.stack_id);

    for (stack_id, documents_group) in documents_by_stack_id {
        // .unwrap() is safe here, because each document is created by an existing stack,
        // the `stacks` HashMap contains all instantiated stacks, and the documents that belong
        // to the exploration stack have been filtered before.
        let stack = stacks.get_mut(&stack_id).unwrap();

        if let Err(error) = stack.update(&documents_group, ranker) {
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

    if let Err(error) = exploration_stack.update(&exploration_docs, ranker) {
        let error = Error::StackOpFailed(error);
        error!("{}", error);
        errors.push(error);
    } else {
        exploration_stack.data.retain_top(keep_top);
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
    ranker: &(impl Ranker + Send + Sync),
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
    let (documents, errors) = documentify_articles(stack.id(), ranker, articles);

    // only return an error if all articles failed
    if documents.is_empty() && !errors.is_empty() {
        Err(Error::Errors(errors))
    } else {
        Ok(documents)
    }
}

fn documentify_articles(
    stack_id: StackId,
    ranker: &(impl Ranker + Send + Sync),
    articles: Vec<Article>,
) -> (Vec<Document>, Vec<Error>) {
    articles
        .into_par_iter()
        .map(|article| {
            let embedding = ranker
                .compute_smbert(article.excerpt_or_title())
                .map_err(|error| {
                    let error = Error::Ranker(error);
                    error!("{}", error);
                    error
                })?;
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

fn documentify_topics(
    ranker: &(impl Ranker + Send + Sync),
    topics: Vec<BingTopic>,
) -> (Vec<TrendingTopic>, Vec<Error>) {
    topics
        .into_par_iter()
        .map(|topic| {
            let embedding = ranker.compute_smbert(&topic.name).map_err(|err| {
                let error = Error::Ranker(err);
                error!("{}", error);
                error
            })?;
            (topic, embedding).try_into().map_err(|err| {
                let error = Error::Document(err);
                error!("{}", error);
                error
            })
        })
        .partition_map(|result| match result {
            Ok(topic) => Either::Left(topic),
            Err(err) => Either::Right(err),
        })
}

/// A discovery engine with [`xayn_discovery_engine_ai::Ranker`] as a ranker.
pub type XaynAiEngine = Engine<xayn_discovery_engine_ai::Ranker>;

impl XaynAiEngine {
    /// Creates a discovery engine with [`xayn_discovery_engine_ai::Ranker`] as a ranker.
    pub async fn from_config(
        config: InitConfig,
        state: Option<&[u8]>,
        history: &[HistoricDocument],
    ) -> Result<Self, Error> {
        let ai_config = ai_config_from_json(config.ai_config.as_deref().unwrap_or("{}"));
        let smbert_config = SMBertConfig::from_files(&config.smbert_vocab, &config.smbert_model)
            .map_err(|err| Error::Ranker(err.into()))?
            .with_token_size(
                ai_config
                    .extract_inner("smbert.token_size")
                    .map_err(|err| Error::Ranker(err.into()))?,
            )
            .map_err(|err| Error::Ranker(err.into()))?
            .with_accents(AccentChars::Cleanse)
            .with_case(CaseChars::Lower)
            .with_pooling::<AveragePooler>();

        let kpe_config = KpeConfig::from_files(
            &config.kpe_vocab,
            &config.kpe_model,
            &config.kpe_cnn,
            &config.kpe_classifier,
        )
        .map_err(|err| Error::Ranker(err.into()))?
        .with_token_size(
            ai_config
                .extract_inner("kpe.token_size")
                .map_err(|err| Error::Ranker(err.into()))?,
        )
        .map_err(|err| Error::Ranker(err.into()))?
        .with_accents(AccentChars::Cleanse)
        .with_case(CaseChars::Keep);

        let coi_system_config = ai_config
            .extract()
            .map_err(|err| Error::Ranker(err.into()))?;

        let builder =
            Builder::from(smbert_config, kpe_config).with_coi_system_config(coi_system_config);

        let client = Arc::new(Client::new(&config.api_key, &config.api_base_url));
        let endpoint_config = config.into();
        let stack_ops = vec![
            Box::new(BreakingNews::new(&endpoint_config, client.clone())) as BoxedOps,
            Box::new(TrustedNews::new(&endpoint_config, client.clone())) as BoxedOps,
            Box::new(PersonalizedNews::new(&endpoint_config, client.clone())) as BoxedOps,
        ];

        if let Some(state) = state {
            let state: State = bincode::deserialize(state).map_err(Error::Deserialization)?;
            let ranker = builder.with_serialized_state(&state.ranker.0)?.build()?;
            Self::from_state(
                &state.engine,
                endpoint_config,
                ranker,
                history,
                stack_ops,
                client,
            )
            .await
        } else {
            let ranker = builder.build()?;
            Self::new(endpoint_config, ranker, history, stack_ops, client).await
        }
    }
}

fn ai_config_from_json(json: &str) -> Figment {
    Figment::new()
        .merge(Serialized::defaults(CoiSystemConfig::default()))
        .merge(Serialized::default("kpe.token_size", 150))
        .merge(Serialized::default("smbert.token_size", 150))
        .merge(Json::string(json))
}

#[derive(Serialize, Deserialize)]
struct StackState(Vec<u8>);

#[derive(Serialize, Deserialize)]
struct RankerState(Vec<u8>);

#[derive(Serialize, Deserialize)]
struct State {
    /// The serialized engine state.
    engine: StackState,
    /// The serialized ranker state.
    ranker: RankerState,
}

/// Active search mode.
enum SearchBy<'a> {
    /// Search by query.
    Query(&'a Filter),
    /// Search by topic.
    Topic(&'a str),
}

#[cfg(test)]
mod tests {
    use crate::{
        ranker,
        stack::{ops::MockOps, Data},
    };

    use std::mem::size_of;
    use wiremock::{
        matchers::{method, path},
        Mock,
        MockServer,
        ResponseTemplate,
    };

    use super::*;

    #[test]
    fn test_ai_config_from_json_default() -> Result<(), GenericError> {
        let ai_config = ai_config_from_json("{}");
        assert_eq!(ai_config.extract_inner::<usize>("kpe.token_size")?, 150);
        assert_eq!(ai_config.extract_inner::<usize>("smbert.token_size")?, 150);
        assert_eq!(
            ai_config.extract::<CoiSystemConfig>()?,
            CoiSystemConfig::default(),
        );
        Ok(())
    }

    #[test]
    fn test_ai_config_from_json_modified() -> Result<(), GenericError> {
        let ai_config = ai_config_from_json(
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

    #[test]
    fn test_usize_not_to_small() {
        assert!(size_of::<usize>() >= size_of::<u32>());
    }

    #[tokio::test]
    async fn test_cross_stack_deduplication() {
        let mock_server = MockServer::start().await;
        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/newscatcher/duplicates.json"));

        Mock::given(method("GET"))
            .and(path("/_lh"))
            .respond_with(tmpl)
            .mount(&mock_server)
            .await;

        let asset_base = "../../discovery_engine_flutter/example/assets/";
        let market = ("US", "en");
        let config = InitConfig {
            api_key: "test-token".to_string(),
            api_base_url: mock_server.uri(),
            markets: vec![market.into()],
            // This triggers the trusted sources stack to also fetch articles
            trusted_sources: vec!["example.com".to_string()],
            excluded_sources: vec![],
            smbert_vocab: format!("{}/smbert_v0001/vocab.txt", asset_base),
            smbert_model: format!("{}/smbert_v0001/smbert-mocked.onnx", asset_base),
            kpe_vocab: format!("{}/kpe_v0001/vocab.txt", asset_base),
            kpe_model: format!("{}/kpe_v0001/bert-mocked.onnx", asset_base),
            kpe_cnn: format!("{}/kpe_v0001/cnn.binparams", asset_base),
            kpe_classifier: format!("{}/kpe_v0001/classifier.binparams", asset_base),
            ai_config: None,
        };
        let endpoint_config = config.clone().into();
        let client = Arc::new(Client::new(&config.api_key, &config.api_base_url));

        // We assume that, if de-duplication works between two stacks, it'll work between
        // any number of stacks. So we just create two.
        let stack_ops = vec![
            Box::new(BreakingNews::new(&endpoint_config, client.clone())) as BoxedOps,
            Box::new(TrustedNews::new(&endpoint_config, client.clone())) as BoxedOps,
        ];

        let breaking_news_id = stack_ops.get(0).unwrap().id();
        let trusted_news_id = stack_ops.get(1).unwrap().id();

        // To test de-duplication we don't really need any ranking, so this
        // this is essentially a no-op ranker.
        let mut no_op_ranker = new_no_op_ranker();
        let mut stacks = create_stacks_from_stack_ops(stack_ops);
        let mut exploration_stack = exploration::Stack::new(StackData::default()).unwrap();

        // Stacks should be empty before we start fetching anything
        assert_eq!(stacks.get(&breaking_news_id).unwrap().len(), 0);
        assert_eq!(stacks.get(&trusted_news_id).unwrap().len(), 0);

        // Update stacks does a lot of things, what's relevant for us is that
        //      a) it fetches new documents
        //      b) it's supposed to de-duplicate between stacks
        // in that order.
        update_stacks(
            &mut stacks,
            &mut exploration_stack,
            &mut no_op_ranker,
            &[],
            10,
            10,
            10,
            &[market.into()],
        )
        .await
        .unwrap();

        // After calling `update_stacks` once, one of the two stacks should contain one document.
        // Both stacks fetched the same item, but de-duplication should prevent the same document
        // being added to both stacks.
        assert_eq!(
            stacks.get(&breaking_news_id).unwrap().len()
                + stacks.get(&trusted_news_id).unwrap().len(),
            1,
        );

        // Now we call `update_stacks` again. We do this to ensure that de-duplication also takes
        // into account the items that are already present inside the stacks, and not only the
        // newly fetched documents.
        update_stacks(
            &mut stacks,
            &mut exploration_stack,
            &mut no_op_ranker,
            &[],
            10,
            10,
            10,
            &[market.into()],
        )
        .await
        .unwrap();

        // No new documents should have been added by the second `update_stacks` call.
        assert_eq!(
            stacks.get(&breaking_news_id).unwrap().len()
                + stacks.get(&trusted_news_id).unwrap().len(),
            1,
        );
    }

    #[tokio::test]
    async fn test_update_stack_no_error_when_no_stack_is_ready() {
        let mut mock_ops = new_mock_stack_ops();
        mock_ops
            .expect_new_items()
            .returning(|_, _, _, _| Err(NewItemsError::NotReady));

        let stack_ops = vec![Box::new(mock_ops) as BoxedOps];
        let mut stacks = create_stacks_from_stack_ops(stack_ops);

        let market = ("US", "en");
        let mut no_op_ranker = new_no_op_ranker();
        let mut exploration_stack = exploration::Stack::new(StackData::default()).unwrap();

        let result = update_stacks(
            &mut stacks,
            &mut exploration_stack,
            &mut no_op_ranker,
            &[],
            10,
            10,
            10,
            &[market.into()],
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_stack_no_error_when_one_stack_is_successful() {
        let mut mock_ops_ok = new_mock_stack_ops();
        mock_ops_ok
            .expect_new_items()
            .returning(|_, _, _, _| Ok(vec![new_mock_article()]));

        let mut mock_ops_failed = new_mock_stack_ops();
        mock_ops_failed
            .expect_new_items()
            .returning(|_, _, _, _| Err(NewItemsError::Error("mock_ops_failed_error".into())));

        let stack_ops = vec![
            Box::new(mock_ops_ok) as BoxedOps,
            Box::new(mock_ops_failed) as BoxedOps,
        ];
        let mut stacks = create_stacks_from_stack_ops(stack_ops);

        let market = ("US", "en");
        let mut no_op_ranker = new_no_op_ranker();
        let mut exploration_stack = exploration::Stack::new(StackData::default()).unwrap();

        let result = update_stacks(
            &mut stacks,
            &mut exploration_stack,
            &mut no_op_ranker,
            &[],
            10,
            10,
            10,
            &[market.into()],
        )
        .await;

        result.unwrap();
    }

    #[tokio::test]
    async fn test_update_stack_should_error_when_all_stacks_fail() {
        let mut mock_ops_failed = new_mock_stack_ops();
        mock_ops_failed
            .expect_new_items()
            .returning(|_, _, _, _| Err(NewItemsError::Error("mock_ops_failed_error".into())));

        let stack_ops = vec![Box::new(mock_ops_failed) as BoxedOps];
        let mut stacks = create_stacks_from_stack_ops(stack_ops);

        let market = ("US", "en");
        let mut no_op_ranker = new_no_op_ranker();
        let mut exploration_stack = exploration::Stack::new(StackData::default()).unwrap();

        let result = update_stacks(
            &mut stacks,
            &mut exploration_stack,
            &mut no_op_ranker,
            &[],
            10,
            10,
            10,
            &[market.into()],
        )
        .await;

        if let Err(crate::Error::Errors(errors)) = result {
            match &errors.as_slice() {
                &[crate::Error::StackOpFailed(stack::Error::New(NewItemsError::Error(msg)))] => {
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
        // We need a mock server from which the initialized stacks can fetch articles
        let mock_server = MockServer::start().await;
        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/newscatcher/duplicates.json"));

        Mock::given(method("GET"))
            .and(path("/_lh"))
            .respond_with(tmpl)
            .mount(&mock_server)
            .await;

        // The config mostly tells the engine were to find the model assets.
        // Here we use the mocked ones, for speed.
        let asset_base = "../../discovery_engine_flutter/example/assets/";
        let config = InitConfig {
            api_key: "test-token".to_string(),
            api_base_url: mock_server.uri(),
            markets: vec![Market {
                country_code: "US".to_string(),
                lang_code: "en".to_string(),
            }],
            // This triggers the trusted sources stack to also fetch articles
            trusted_sources: vec!["example.com".to_string()],
            excluded_sources: vec![],
            smbert_vocab: format!("{}/smbert_v0001/vocab.txt", asset_base),
            smbert_model: format!("{}/smbert_v0001/smbert-mocked.onnx", asset_base),
            kpe_vocab: format!("{}/kpe_v0001/vocab.txt", asset_base),
            kpe_model: format!("{}/kpe_v0001/bert-mocked.onnx", asset_base),
            kpe_cnn: format!("{}/kpe_v0001/cnn.binparams", asset_base),
            kpe_classifier: format!("{}/kpe_v0001/classifier.binparams", asset_base),
            ai_config: None,
        };

        // Now we can initialize the engine with no previous history or state. This should
        // be the same as when it's initialized for the first time after the app is downloaded.
        let state = None;
        let history = &[];
        let mut engine = XaynAiEngine::from_config(config, state, history)
            .await
            .unwrap();

        // Finally, we instruct the engine to fetch some articles and check whether or not
        // the expected articles from the mock show up in the results.
        let res = engine.get_feed_documents(history, 2).await.unwrap();

        assert_eq!(1, res.len());
        assert_eq!(
            res.get(0).unwrap().resource.title,
            "Some really important article",
        );
    }

    fn new_no_op_ranker() -> impl Ranker {
        let mut ranker = ranker::MockRanker::new();
        ranker.expect_rank().returning(|_: &mut [Document]| Ok(()));
        ranker.expect_take_key_phrases().returning(|_, _| vec![]);
        ranker.expect_positive_cois().return_const(vec![]);
        ranker.expect_negative_cois().return_const(vec![]);
        ranker.expect_compute_smbert().returning(|_| {
            let embedding: Embedding = [0.0].into();
            Ok(embedding)
        });
        ranker
    }

    fn create_stacks_from_stack_ops(stack_ops: Vec<BoxedOps>) -> HashMap<Id, Stack> {
        stack_ops
            .into_iter()
            .map(|ops| {
                let data = Data::new(1.0, 1.0, vec![]).unwrap();
                let stack = Stack::new(data, ops).unwrap();
                (stack.id(), stack)
            })
            .collect()
    }

    fn new_mock_article() -> Article {
        serde_json::from_str(r#"{"link": "https://xayn.com"}"#).unwrap()
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
}
