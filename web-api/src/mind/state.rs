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

use anyhow::Error;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::Serialize;
use xayn_ai_bert::NormalizedEmbedding;
use xayn_ai_coi::{nan_safe_f32_cmp_desc, CoiConfig, CoiSystem};

use crate::{
    embedding::{self, Embedder},
    mind::{config::StateConfig, data::Document},
    models::{DocumentId, DocumentProperties, IngestedDocument, UserId, UserInteractionType},
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
    pub(super) fn new(storage: Storage, config: StateConfig) -> Result<Self, Error> {
        let embedder = Embedder::load(&embedding::Config {
            directory: "../assets/smbert_v0003".into(),
            ..embedding::Config::default()
        })?;

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

    pub(super) async fn insert(&self, documents: Vec<Document>) -> Result<(), Error> {
        let documents = documents
            .into_iter()
            .map(|document| {
                let document = IngestedDocument {
                    id: document.id,
                    snippet: document.snippet,
                    properties: DocumentProperties::default(),
                    tags: vec![document.category, document.subcategory],
                };
                let embedding = self.embedder.run(&document.snippet)?;

                Ok((document, embedding))
            })
            .try_collect::<_, _, Error>()?;

        storage::Document::insert(&self.storage, documents)
            .await
            .map_err(Into::into)
    }

    #[allow(dead_code)]
    pub(super) async fn update(
        &self,
        embeddings: Vec<(DocumentId, NormalizedEmbedding)>,
    ) -> Result<(), Error> {
        let mut documents =
            storage::Document::get_personalized(&self.storage, embeddings.iter().map(|(id, _)| id))
                .await?
                .into_iter()
                .map(|document| (document.id.clone(), document))
                .collect::<HashMap<_, _>>();
        let documents = embeddings
            .into_iter()
            .map(|(id, embedding)| {
                let document = documents.remove(&id).unwrap(/* document must already exist */);
                let document = IngestedDocument {
                    id,
                    snippet: String::new(/* unused for in-memory db */),
                    properties: document.properties,
                    tags: document.tags,
                };
                (document, embedding)
            })
            .collect_vec();
        storage::Document::insert(&self.storage, documents).await?;

        Ok(())
    }

    pub(super) async fn interact(
        &self,
        user: &UserId,
        documents: impl IntoIterator<Item = (&DocumentId, DateTime<Utc>)>,
    ) -> Result<(), Error> {
        for (id, time) in documents {
            update_interactions(
                &self.storage,
                &self.coi,
                user,
                [&(id.clone(), UserInteractionType::Positive)],
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
    ) -> Result<Option<Vec<DocumentId>>, Error> {
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

/// The results of iteration of the saturation benchmark
#[derive(Debug, Default, Serialize)]
pub(super) struct SaturationIteration {
    pub(super) shown_documents: Vec<DocumentId>,
    pub(super) clicked_documents: Vec<DocumentId>,
}

/// The results of the saturation benchmark for one topic
#[derive(Debug, Default, Serialize)]
pub(super) struct SaturationTopicResult {
    pub(super) topic: String,
    pub(super) iterations: Vec<SaturationIteration>,
}

impl SaturationTopicResult {
    pub(super) fn new(topic: &str, iterations: usize) -> Self {
        Self {
            topic: topic.to_owned(),
            iterations: Vec::with_capacity(iterations),
        }
    }
}

/// The results of the saturation benchmark
#[derive(Debug, Default, Serialize)]
pub(super) struct SaturationResult {
    pub(super) topics: Vec<SaturationTopicResult>,
}

impl SaturationResult {
    pub(super) fn new(topics: usize) -> Self {
        Self {
            topics: Vec::with_capacity(topics),
        }
    }
}

pub(super) fn ndcg(relevance: &[f32], k: &[usize]) -> Vec<f32> {
    let mut optimal_order = relevance.to_owned();
    optimal_order.sort_by(nan_safe_f32_cmp_desc);
    let last = k
        .iter()
        .max()
        .copied()
        .map_or_else(|| relevance.len(), |k| k.min(relevance.len()));

    let ndcgs = relevance
        .iter()
        .zip(optimal_order)
        .take(last)
        .scan(
            (1_f32, 0., 0.),
            |(i, dcg, ideal_dcg), (relevance, optimal_order)| {
                *i += 1.;
                let log_i = (*i).log2();
                *dcg += (2_f32.powf(*relevance) - 1.) / log_i;
                *ideal_dcg += (2_f32.powf(optimal_order) - 1.) / log_i;
                Some(*dcg / (*ideal_dcg + 0.00001))
            },
        )
        .collect::<Vec<_>>();

    k.iter()
        .map(|nrank| match ndcgs.get(*nrank - 1) {
            Some(i) => i,
            None => ndcgs.last().unwrap(),
        })
        .copied()
        .collect::<Vec<_>>()
}
