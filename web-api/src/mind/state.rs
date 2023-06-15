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

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use derive_more::{Deref, DerefMut};
use itertools::Itertools;
use serde::Serialize;
use xayn_ai_bert::NormalizedEmbedding;
use xayn_ai_coi::{CoiConfig, CoiSystem};
use xayn_test_utils::error::Panic;

use crate::{
    embedding::{self, Embedder},
    mind::{config::StateConfig, data::Document},
    models::{DocumentId, DocumentProperties, DocumentSnippet, IngestedDocument, UserId},
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
    pub(super) fn new(storage: Storage, config: StateConfig) -> Result<Self, Panic> {
        let embedder = Embedder::load(&embedding::Config {
            directory: "../assets/xaynia_v0002".into(),
            ..embedding::Config::default()
        })
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
            .map(|document| {
                let embedding = self.embedder.run(&document.snippet)?;
                Ok(IngestedDocument {
                    id: document.id,
                    snippet: document.snippet,
                    properties: DocumentProperties::default(),
                    tags: vec![document.category, document.subcategory].try_into()?,
                    embedding,
                    is_candidate: true,
                })
            })
            .try_collect::<_, _, Panic>()?;
        storage::Document::insert(&self.storage, documents).await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub(super) async fn update(
        &self,
        embeddings: Vec<(DocumentId, NormalizedEmbedding)>,
    ) -> Result<(), Panic> {
        let mut documents =
            storage::Document::get_personalized(&self.storage, embeddings.iter().map(|(id, _)| id))
                .await?
                .into_iter()
                .map(|document| (document.id.clone(), document))
                .collect::<HashMap<_, _>>();
        let snippet = DocumentSnippet::try_from("snippet" /* unused for in-memory db */)?;
        let documents = embeddings
            .into_iter()
            .map(|(id, embedding)| {
                let document = documents.remove(&id).unwrap(/* document must already exist */);
                IngestedDocument {
                    id,
                    snippet: snippet.clone(),
                    properties: document.properties,
                    tags: document.tags,
                    embedding,
                    is_candidate: true,
                }
            })
            .collect_vec();
        storage::Document::insert(&self.storage, documents).await?;

        Ok(())
    }

    pub(super) async fn interact(
        &self,
        user: &UserId,
        documents: impl IntoIterator<Item = (&DocumentId, DateTime<Utc>)>,
    ) -> Result<(), Panic> {
        for (id, time) in documents {
            update_interactions(
                &self.storage,
                &self.coi,
                user,
                [id],
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
        )
        .await
        .map(|documents| {
            documents.map(|documents| documents.into_iter().map(|document| document.id).collect())
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
