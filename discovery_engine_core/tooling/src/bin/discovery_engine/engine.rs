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

use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rand::{thread_rng, Rng};
use xayn_discovery_engine_core::{
    document::{Document, TimeSpent, UserReacted, UserReaction, ViewMode},
    Engine,
    InitConfig,
};
use xayn_discovery_engine_providers::Market;
use xayn_discovery_engine_test_utils::asset::smbert_quantized;

use crate::io::{Dislike, Dislikes, Like, Likes, Output, Persona, Personas};

pub(crate) struct TestEngine {
    engine: Engine,
    progress: bool,
}

impl TestEngine {
    pub(crate) async fn new(api_key: String, progress: bool) -> Result<Self> {
        let spinner = progress_spinner(progress, "initializing engine");

        let config = InitConfig {
            api_key,
            api_base_url: "https://api-gw.xaynet.dev".into(),
            news_provider: None,
            similar_news_provider: None,
            headlines_provider: None,
            trusted_headlines_provider: None,
            markets: vec![Market::new("de", "DE"), Market::new("en", "US")],
            bert: smbert_quantized()?.display().to_string(),
            max_docs_per_feed_batch: 1,
            max_docs_per_search_batch: 1,
            de_config: None,
            log_file: None,
            data_dir: String::new(),
            use_ephemeral_db: true,
            dart_migration_data: None,
        };
        let engine = Engine::from_config(config).await?.0;

        spinner.finish_with_message("initialized engine");

        Ok(Self { engine, progress })
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
                document = self
                    .engine
                    .user_reacted(UserReacted {
                        id: document.id,
                        reaction: UserReaction::Positive,
                    })
                    .await?;
                let time_spent = TimeSpent {
                    id: document.id,
                    view_time: *time_spent,
                    view_mode: ViewMode::Story,
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
                document = self
                    .engine
                    .user_reacted(UserReacted {
                        id: document.id,
                        reaction: UserReaction::Negative,
                    })
                    .await?;
            }
        }

        Ok(document)
    }

    async fn reset(&mut self, like_topics: &Likes, dislike_topics: &Dislikes) -> Result<()> {
        self.engine.reset_ai().await?;

        let mut cois = 0;
        while cois < self.engine.coi_config().min_positive_cois() {
            if let Some(document) = self.engine.feed_next_batch().await?.pop() {
                self.like(document, like_topics, true).await?;
                cois += 1;
            }
        }
        cois = 0;
        while cois < self.engine.coi_config().min_negative_cois() {
            if let Some(document) = self.engine.feed_next_batch().await?.pop() {
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
        let multi_bar = MultiProgress::new();
        let personas_bar =
            multi_progress_bar(&multi_bar, self.progress, personas.len(), "personas");
        for (
            name,
            Persona {
                like_topics,
                dislike_topics,
                trusted_sources,
                excluded_sources,
            },
        ) in personas_bar.wrap_iter(personas.into_iter())
        {
            self.engine
                .set_sources(trusted_sources, excluded_sources)
                .await?;

            let mut documents = Vec::with_capacity(runs * iterations);
            let runs_bar = multi_progress_bar(&multi_bar, self.progress, runs, "runs");
            for _ in runs_bar.wrap_iter(0..runs) {
                let spinner = multi_progress_spinner(&multi_bar, self.progress, "resetting engine");
                self.reset(&like_topics, &dislike_topics).await?;
                spinner.finish();
                multi_bar.remove(&spinner);

                let iterations_bar =
                    multi_progress_bar(&multi_bar, self.progress, iterations, "iterations");
                for _ in iterations_bar.wrap_iter(0..iterations) {
                    if let Some(document) = self.engine.feed_next_batch().await?.pop() {
                        let document = self.like(document, &like_topics, false).await?;
                        let document = self.dislike(document, &dislike_topics, false).await?;
                        documents.push(document.into());
                    }
                }
                multi_bar.remove(&iterations_bar);
            }
            multi_bar.remove(&runs_bar);
            output.insert(name, documents);
        }
        personas_bar.finish();

        Ok(Output(output))
    }
}

fn progress_bar(progress: bool, len: usize, msg: &'static str) -> ProgressBar {
    progress
        .then(|| ProgressBar::new(len as u64))
        .unwrap_or_else(ProgressBar::hidden)
        .with_message(msg)
        .with_style(ProgressStyle::with_template("{bar:50} {pos}/{len} {msg}").unwrap())
}

fn multi_progress_bar(
    multi: &MultiProgress,
    progress: bool,
    len: usize,
    msg: &'static str,
) -> ProgressBar {
    let bar = progress_bar(progress, len, msg);
    if progress {
        multi.add(bar)
    } else {
        bar
    }
}

fn progress_spinner(progress: bool, msg: &'static str) -> ProgressBar {
    let spinner = progress
        .then(ProgressBar::new_spinner)
        .unwrap_or_else(ProgressBar::hidden)
        .with_message(msg);
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner
}

fn multi_progress_spinner(multi: &MultiProgress, progress: bool, msg: &'static str) -> ProgressBar {
    let spinner = progress_spinner(progress, msg);
    if progress {
        multi.add(spinner)
    } else {
        spinner
    }
}
