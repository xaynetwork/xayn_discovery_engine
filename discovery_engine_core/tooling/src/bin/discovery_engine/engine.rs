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

#![allow(dead_code)]

use std::collections::HashMap;

use anyhow::Result;
use rand::{thread_rng, Rng};
use xayn_discovery_engine_core::{
    document::{Document, TimeSpent, UserReacted, UserReaction, ViewMode},
    Engine,
    InitConfig,
};
use xayn_discovery_engine_providers::Market;

use crate::io::{Dislike, Dislikes, Like, Likes, Output, Persona, Personas};

pub(crate) struct TestEngine {
    engine: Engine,
}

impl TestEngine {
    pub(crate) async fn new(api_key: String) -> Result<Self> {
        let asset_base = "../discovery_engine_flutter/example/assets";
        let config = InitConfig {
            api_key,
            api_base_url: "https://api-gw.xaynet.dev".into(),
            news_provider_path: "/newscatcher/v1/search-news".into(),
            headlines_provider_path: "/newscatcher/v1/latest-headlines".into(),
            markets: vec![Market::new("de", "DE"), Market::new("en", "US")],
            trusted_sources: vec![],
            excluded_sources: vec![],
            smbert_vocab: format!("{asset_base}/smbert_v0001/vocab.txt"),
            smbert_model: format!("{asset_base}/smbert_v0001/smbert-quantized.onnx"),
            kpe_vocab: format!("{asset_base}/kpe_v0001/vocab.txt"),
            kpe_model: format!("{asset_base}/kpe_v0001/bert-quantized.onnx"),
            kpe_cnn: format!("{asset_base}/kpe_v0001/cnn.binparams"),
            kpe_classifier: format!("{asset_base}/kpe_v0001/classifier.binparams"),
            max_docs_per_feed_batch: 1,
            max_docs_per_search_batch: 1,
            de_config: None,
            log_file: None,
            data_dir: String::new(),
            use_in_memory_db: true,
        };
        let engine = Engine::from_config(config, None, &[], &[]).await?;

        Ok(Self { engine })
    }

    async fn like(
        &mut self,
        mut document: Document,
        topics: &Likes,
        certainly: bool,
    ) -> Result<Document> {
        if let Some(Like {
            probability,
            time_spent,
        }) = topics.get(&document.resource.topic)
        {
            if certainly || thread_rng().gen_bool(*probability) {
                let reacted = user_reacted(document, UserReaction::Positive);
                document = self
                    .engine
                    .user_reacted(
                        Some(&[/* TODO: db migration */]),
                        &[/* TODO: db migration */],
                        reacted,
                    )
                    .await?;
                let time_spent = TimeSpent {
                    id: document.id,
                    smbert_embedding: document.smbert_embedding.clone(),
                    view_time: *time_spent,
                    view_mode: ViewMode::Story,
                    reaction: UserReaction::Positive,
                };
                self.engine.time_spent(time_spent).await?;
            }
        }

        Ok(document)
    }

    async fn dislike(
        &mut self,
        mut document: Document,
        topics: &Dislikes,
        certainly: bool,
    ) -> Result<Document> {
        if let Some(Dislike { probability }) = topics.get(&document.resource.topic) {
            if certainly || thread_rng().gen_bool(*probability) {
                let reacted = user_reacted(document, UserReaction::Negative);
                document = self
                    .engine
                    .user_reacted(None, &[/* TODO: db migration */], reacted)
                    .await?;
            }
        }

        Ok(document)
    }

    async fn reset(&mut self, like_topics: &Likes, dislike_topics: &Dislikes) -> Result<()> {
        self.engine.reset_ai().await?;

        let mut cois = 0;
        while cois < self.engine.coi_system_config().min_positive_cois() {
            if let Some(document) = self
                .engine
                .get_feed_documents(&[/* TODO: db migration */], &[/* TODO: db migration */])
                .await?
                .pop()
            {
                self.like(document, like_topics, true).await?;
                cois += 1;
            }
        }
        cois = 0;
        while cois < self.engine.coi_system_config().min_negative_cois() {
            if let Some(document) = self
                .engine
                .get_feed_documents(&[/* TODO: db migration */], &[/* TODO: db migration */])
                .await?
                .pop()
            {
                self.dislike(document, dislike_topics, true).await?;
                cois += 1;
            }
        }

        Ok(())
    }

    pub(crate) async fn run(
        &mut self,
        runs: usize,
        iterations: usize,
        personas: Personas,
    ) -> Result<Output> {
        let mut output = HashMap::with_capacity(personas.len());
        for (
            name,
            Persona {
                like_topics,
                dislike_topics,
                trusted_sources,
                excluded_sources,
            },
        ) in personas
        {
            self.engine
                .set_trusted_sources(
                    &[/* TODO: db migration */],
                    &[/* TODO: db migration */],
                    trusted_sources,
                )
                .await?;
            self.engine
                .set_excluded_sources(
                    &[/* TODO: db migration */],
                    &[/* TODO: db migration */],
                    excluded_sources,
                )
                .await?;

            let mut documents = Vec::with_capacity(runs * iterations);
            for _ in 0..runs {
                self.reset(&like_topics, &dislike_topics).await?;
                for _ in 0..iterations {
                    if let Some(document) = self
                        .engine
                        .get_feed_documents(
                            &[/* TODO: db migration */],
                            &[/* TODO: db migration */],
                        )
                        .await?
                        .pop()
                    {
                        let document = self.like(document, &like_topics, false).await?;
                        let document = self.dislike(document, &dislike_topics, false).await?;
                        documents.push(document.into());
                    }
                }
            }
            output.insert(name, documents);
        }

        Ok(Output(output))
    }
}

fn user_reacted(document: Document, reaction: UserReaction) -> UserReacted {
    UserReacted {
        id: document.id,
        stack_id: document.stack_id,
        title: document.resource.title,
        snippet: document.resource.snippet,
        smbert_embedding: document.smbert_embedding,
        reaction,
        market: Market::new(document.resource.language, document.resource.country),
    }
}
