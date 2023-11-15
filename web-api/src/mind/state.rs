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

use chrono::{DateTime, Utc};
use derive_more::{Deref, DerefMut};
use futures_util::{stream::FuturesOrdered, TryStreamExt};
use serde::Serialize;
use xayn_ai_coi::{CoiConfig, CoiSystem};
use xayn_test_utils::{
    asset::{ort, xaynia},
    error::Panic,
};

use crate::{
    embedding::{self, Embedder, EmbeddingKind, Pipeline},
    mind::{config::StateConfig, data::Document},
    models::{
        DocumentContent,
        DocumentForIngestion,
        DocumentId,
        DocumentProperties,
        PreprocessingStep,
        Sha256Hash,
        SnippetOrDocumentId,
        UserId,
    },
    personalization::{
        routes::{personalize_documents_by, update_interactions, PersonalizeBy},
        PersonalizationConfig,
    },
    storage::{self, memory::Storage},
};

pub(super) struct State {
    storage: Storage,
    embedder: Embedder,
    pub(super) coi: CoiSystem,
    personalization: PersonalizationConfig,
    pub(super) time: DateTime<Utc>,
}

impl State {
    pub(super) async fn new(storage: Storage, config: StateConfig) -> Result<Self, Panic> {
        let embedder = Embedder::load(&embedding::Config::Pipeline(Pipeline {
            directory: xaynia()?.into(),
            runtime: ort()?.into(),
            ..Pipeline::default()
        }))
        .await
        .map_err(|error| Panic::from(&*error))?;

        let coi = config.coi.build();
        let personalization = config.personalization;
        let time = config.time;

        Ok(Self {
            storage,
            embedder,
            coi,
            personalization,
            time,
        })
    }

    pub(super) fn with_coi_config(&mut self, config: CoiConfig) {
        self.coi = config.build();
    }

    pub(super) async fn insert(&self, documents: Vec<Document>) -> Result<(), Panic> {
        let documents = documents
            .into_iter()
            .map(|document| async move {
                let embedding = self
                    .embedder
                    .run(EmbeddingKind::Content, &document.snippet)
                    .await?;
                Ok::<_, Panic>(DocumentForIngestion {
                    id: document.id,
                    original_sha256: Sha256Hash::calculate(document.snippet.as_bytes()),
                    snippets: vec![DocumentContent {
                        snippet: document.snippet,
                        embedding,
                    }],
                    preprocessing_step: PreprocessingStep::None,
                    properties: DocumentProperties::default(),
                    tags: vec![document.category, document.subcategory].try_into()?,
                    is_candidate: true,
                })
            })
            .collect::<FuturesOrdered<_>>()
            .try_collect()
            .await?;

        storage::Document::insert(&self.storage, documents).await?;

        Ok(())
    }

    pub(super) async fn interact(
        &self,
        user: &UserId,
        documents: impl IntoIterator<Item = (SnippetOrDocumentId, DateTime<Utc>)>,
    ) -> Result<(), Panic> {
        for (id, time) in documents {
            update_interactions(
                &self.storage,
                &self.coi,
                user,
                vec![id],
                self.personalization.store_user_history,
                time,
            )
            .await?;
        }

        Ok(())
    }

    pub(super) async fn personalize(
        &self,
        user: &UserId,
        by: PersonalizeBy<'_>,
        time: DateTime<Utc>,
    ) -> Result<Option<Vec<DocumentId>>, Panic> {
        personalize_documents_by(
            &self.storage,
            &self.coi,
            user,
            &self.personalization,
            by,
            time,
            false,
            false,
        )
        .await
        .map(|documents| {
            documents.map(|documents| {
                documents
                    .into_iter()
                    .map(|document| document.id.into_document_id())
                    .collect()
            })
        })
        .map_err(Into::into)
    }
}

/// The results of iteration of the saturation benchmark.
#[derive(Debug, Default, Serialize)]
pub(super) struct SaturationIteration {
    pub(super) shown_documents: Vec<DocumentId>,
    pub(super) clicked_documents: Vec<DocumentId>,
}

/// The results of the saturation benchmark for one topic.
#[derive(Debug, Default, Deref, DerefMut, Serialize)]
pub(super) struct SaturationTopicResult {
    topic: String,
    #[deref]
    #[deref_mut]
    iterations: Vec<SaturationIteration>,
}

impl SaturationTopicResult {
    pub(super) fn new(topic: &str, iterations: usize) -> Self {
        Self {
            topic: topic.to_string(),
            iterations: Vec::with_capacity(iterations),
        }
    }
}

/// The results of the saturation benchmark.
#[derive(Debug, Default, Deref, DerefMut, Serialize)]
pub(super) struct SaturationResult {
    topics: Vec<SaturationTopicResult>,
}

impl SaturationResult {
    pub(super) fn new(topics: usize) -> Self {
        Self {
            topics: Vec::with_capacity(topics),
        }
    }
}
