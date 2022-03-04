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

use std::{collections::HashMap, sync::Arc};

use displaydoc::Display;
use figment::{
    providers::{Format, Json, Serialized},
    Figment,
};
use rayon::iter::{Either, IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::error;

use xayn_ai::{
    ranker::{AveragePooler, Builder, CoiSystemConfig},
    KpeConfig,
    SMBertConfig,
};
use xayn_discovery_engine_providers::Market;

use crate::{
    document::{
        self,
        document_from_article,
        Document,
        HistoricDocument,
        TimeSpent,
        UserReacted,
        UserReaction,
    },
    mab::{self, BetaSampler, SelectionIter},
    ranker::Ranker,
    stack::{
        self,
        BoxedOps,
        BreakingNews,
        Data as StackData,
        Id as StackId,
        PersonalizedNews,
        Stack,
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

    /// List of errors/warnings. {0:?}
    Errors(Vec<Error>),
}

/// Configuration settings to initialize Discovery Engine with a [`xayn_ai::ranker::Ranker`].
pub struct InitConfig {
    /// Key for accessing the API.
    pub api_key: String,
    /// API base url.
    pub api_base_url: String,
    /// List of markets to use.
    pub markets: Vec<Market>,
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
pub struct EndpointConfig {
    /// Key for accessing API.
    pub(crate) api_key: String,
    /// Base URL for API.
    pub(crate) api_base_url: String,
    /// Page size setting for API.
    pub(crate) page_size: usize,
    /// Write-exclusive access to markets list.
    pub(crate) markets: Arc<RwLock<Vec<Market>>>,
}

impl From<InitConfig> for EndpointConfig {
    fn from(config: InitConfig) -> Self {
        Self {
            api_key: config.api_key,
            api_base_url: config.api_base_url,
            page_size: 100,
            markets: Arc::new(RwLock::new(config.markets)),
        }
    }
}

/// Temporary config to allow for configurations within the core without a mirroring outside impl.
struct CoreConfig {
    /// The number of selected top key phrases while updating the stacks.
    select_top: usize,
    /// The number of top documents per stack to keep while filtering the stacks.
    keep_top: usize,
    /// The lower bound of documents per stack at which new items are requested.
    request_new: usize,
    /// The number of times to get feed documents after which the stacks are updated without the
    /// limitation of `request_new`.
    request_after: usize,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            select_top: 3,
            keep_top: 20,
            request_new: 3,
            request_after: 2,
        }
    }
}

/// Discovery Engine.
pub struct Engine<R> {
    config: EndpointConfig,
    core_config: CoreConfig,
    stacks: RwLock<HashMap<StackId, Stack>>,
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
    ) -> Result<Self, Error> {
        let stack_data = |_| StackData::default();

        Self::from_stack_data(config, ranker, history, stack_data, stack_ops).await
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
    ) -> Result<Self, Error> {
        if stack_ops.is_empty() {
            return Err(Error::NoStackOps);
        }

        let mut stack_data = bincode::deserialize::<HashMap<StackId, _>>(&state.0)
            .map_err(Error::Deserialization)?;
        let stack_data = |id| stack_data.remove(&id).unwrap_or_default();

        Self::from_stack_data(config, ranker, history, stack_data, stack_ops).await
    }

    async fn from_stack_data(
        config: EndpointConfig,
        mut ranker: R,
        history: &[HistoricDocument],
        mut stack_data: impl FnMut(StackId) -> StackData + Send,
        stack_ops: Vec<BoxedOps>,
    ) -> Result<Self, Error> {
        let mut stacks = stack_ops
            .into_iter()
            .map(|mut ops| {
                let id = ops.id();
                let data = stack_data(id);
                ops.configure(&config);
                Stack::new(data, ops).map(|stack| (id, stack))
            })
            .collect::<Result<HashMap<_, _>, _>>()
            .map_err(Error::InvalidStack)?;
        let core_config = CoreConfig::default();

        // we don't want to fail initialization if there are network problems
        update_stacks(
            stacks.values_mut(),
            &mut ranker,
            history,
            core_config.select_top,
            core_config.keep_top,
            usize::MAX,
        )
        .await
        .ok();

        Ok(Self {
            config,
            core_config,
            stacks: RwLock::new(stacks),
            ranker,
            request_after: 0,
        })
    }

    /// Serializes the state of the `Engine` and `Ranker` state.
    pub async fn serialize(&self) -> Result<Vec<u8>, Error> {
        let stacks = self.stacks.read().await;
        let stacks_data = stacks
            .iter()
            .map(|(id, stack)| (id, &stack.data))
            .collect::<HashMap<_, _>>();

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
        markets: Vec<Market>,
    ) -> Result<(), Error> {
        *self.config.markets.write().await = markets;

        let mut stacks = self.stacks.write().await;
        for stack in stacks.values_mut() {
            stack.data = StackData::default();
        }
        update_stacks(
            stacks.values_mut(),
            &mut self.ranker,
            history,
            self.core_config.select_top,
            self.core_config.keep_top,
            self.core_config.request_new,
        )
        .await
    }

    /// Returns at most `max_documents` [`Document`]s for the feed.
    pub async fn get_feed_documents(
        &mut self,
        history: &[HistoricDocument],
        max_documents: usize,
    ) -> Result<Vec<Document>, Error> {
        let mut stacks = self.stacks.write().await;
        let documents =
            SelectionIter::new(BetaSampler, stacks.values_mut()).select(max_documents)?;

        let request_new = (self.request_after < self.core_config.request_after)
            .then(|| self.core_config.request_new)
            .unwrap_or(usize::MAX);
        update_stacks(
            stacks.values_mut(),
            &mut self.ranker,
            history,
            self.core_config.select_top,
            self.core_config.keep_top,
            request_new,
        )
        .await?;
        self.request_after = (self.request_after + 1) % self.core_config.request_after;

        Ok(documents)
    }

    /// Process the feedback about the user spending some time on a document.
    pub async fn time_spent(&mut self, time_spent: &TimeSpent) -> Result<(), Error> {
        self.ranker.log_document_view_time(time_spent)?;

        rank_stacks(self.stacks.write().await.values_mut(), &mut self.ranker)
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
        stacks
            .get_mut(&reacted.stack_id)
            .ok_or(Error::InvalidStackId(reacted.stack_id))?
            .update_relevance(reacted.reaction);

        self.ranker.log_user_reaction(reacted)?;

        rank_stacks(stacks.values_mut(), &mut self.ranker)?;
        if let UserReaction::Positive = reacted.reaction {
            if let Some(history) = history {
                update_stacks(
                    stacks.values_mut(),
                    &mut self.ranker,
                    history,
                    self.core_config.select_top,
                    self.core_config.keep_top,
                    usize::MAX,
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

    /// Performs an active search with the given query parameters.
    pub async fn active_search(
        &mut self,
        _query: &str,
        _page: usize,
        _page_size: usize,
    ) -> Result<Vec<Document>, Error> {
        todo!() // implemented in TY-2434
    }
}

/// The ranker could rank the documents in a different order so we update the stacks with it.
fn rank_stacks<'a>(
    stacks: impl Iterator<Item = &'a mut Stack>,
    ranker: &mut impl Ranker,
) -> Result<(), Error> {
    let errors = stacks.fold(Vec::new(), |mut errors, stack| {
        if let Err(error) = stack.rank(ranker) {
            errors.push(Error::StackOpFailed(error));
        }

        errors
    });

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::Errors(errors))
    }
}

/// Updates the stacks with data related to the top key phrases of the current data.
async fn update_stacks<'a>(
    stacks: impl Iterator<Item = &'a mut Stack> + Send + Sync,
    ranker: &mut (impl Ranker + Send + Sync),
    history: &[HistoricDocument],
    select_top: usize,
    keep_top: usize,
    request_new: usize,
) -> Result<(), Error> {
    let mut stacks: Vec<_> = stacks.filter(|stack| stack.len() <= request_new).collect();

    let key_phrases = stacks
        .iter()
        .any(|stack| stack.ops.needs_key_phrases())
        .then(|| ranker.select_top_key_phrases(select_top))
        .unwrap_or_default();

    let mut errors = Vec::new();
    for stack in &mut stacks {
        let articles = match stack.new_items(&key_phrases).await {
            Ok(articles) => articles,
            Err(error) => {
                let error = Error::StackOpFailed(error);
                error!("{}", error);
                errors.push(error);
                continue;
            }
        };

        let articles = match stack.filter_articles(history, articles) {
            Ok(articles) => articles,
            Err(error) => {
                let error = Error::StackOpFailed(error);
                error!("{}", error);
                errors.push(error);
                continue;
            }
        };

        let id = stack.id();
        let articles_len = articles.len();
        let (documents, articles_errors) = articles
            .into_par_iter()
            .map(|article| {
                let title = article.title.as_str();
                let embedding = ranker.compute_smbert(title).map_err(|error| {
                    let error = Error::Ranker(error);
                    error!("{}", error);
                    error
                })?;
                document_from_article(article, id, embedding).map_err(|error| {
                    let error = Error::Document(error);
                    error!("{}", error);
                    error
                })
            })
            .partition_map::<Vec<_>, Vec<_>, _, _, _>(|result| match result {
                Ok(document) => Either::Left(document),
                Err(error) => Either::Right(error),
            });
        // only push an error if the articles aren't empty for other reasons and all articles failed
        if articles_len > 0 && articles_errors.len() == articles_len {
            errors.push(Error::Errors(articles_errors));
            continue;
        }

        if let Err(error) = stack.update(&documents, ranker) {
            let error = Error::StackOpFailed(error);
            error!("{}", error);
            errors.push(error);
        } else {
            stack.data.retain_top(keep_top);
        }
    }

    if errors.len() < stacks.len() {
        Ok(())
    } else {
        Err(Error::Errors(errors))
    }
}

/// A discovery engine with [`xayn_ai::ranker::Ranker`] as a ranker.
pub type XaynAiEngine = Engine<xayn_ai::ranker::Ranker>;

impl XaynAiEngine {
    /// Creates a discovery engine with [`xayn_ai::ranker::Ranker`] as a ranker.
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
            .with_accents(false)
            .with_lowercase(true)
            .with_pooling(AveragePooler);

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
        .with_accents(false)
        .with_lowercase(false);

        let coi_system_config = ai_config
            .extract()
            .map_err(|err| Error::Ranker(err.into()))?;

        let builder =
            Builder::from(smbert_config, kpe_config).with_coi_system_config(coi_system_config);

        let stack_ops = vec![
            Box::new(BreakingNews::default()) as BoxedOps,
            Box::new(PersonalizedNews::default()) as BoxedOps,
        ];

        if let Some(state) = state {
            let state: State = bincode::deserialize(state).map_err(Error::Deserialization)?;
            let ranker = builder
                .with_serialized_state(&state.ranker.0)
                .map_err(|err| Error::Ranker(err.into()))?
                .build()
                .map_err(|err| Error::Ranker(err.into()))?;
            Self::from_state(&state.engine, config.into(), ranker, history, stack_ops).await
        } else {
            let ranker = builder.build().map_err(|err| Error::Ranker(err.into()))?;
            Self::new(config.into(), ranker, history, stack_ops).await
        }
    }
}

fn ai_config_from_json(json: &str) -> Figment {
    Figment::new()
        .merge(Serialized::defaults(CoiSystemConfig::default()))
        .merge(Serialized::default("kpe.token_size", 150))
        .merge(Serialized::default("smbert.token_size", 52))
        .merge(Json::string(json))
}

/// A wrapper around a dynamic error type, similar to `anyhow::Error`,
/// but without the need to declare `anyhow` as a dependency.
pub(crate) type GenericError = Box<dyn std::error::Error + Sync + Send + 'static>;

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

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn test_ai_config_from_json_default() -> Result<(), Box<dyn Error>> {
        let ai_config = ai_config_from_json("{}");
        assert_eq!(ai_config.extract_inner::<usize>("kpe.token_size")?, 150);
        assert_eq!(ai_config.extract_inner::<usize>("smbert.token_size")?, 52);
        assert_eq!(
            ai_config.extract::<CoiSystemConfig>()?,
            CoiSystemConfig::default(),
        );
        Ok(())
    }

    #[test]
    fn test_ai_config_from_json_modified() -> Result<(), Box<dyn Error>> {
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
}
