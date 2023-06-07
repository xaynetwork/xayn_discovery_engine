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

//! Executes the MIND benchmarks.

mod config;
mod data;
mod state;

use std::{fs::File, io, io::Write, slice};

use chrono::Duration;
use itertools::Itertools;
use ndarray::s;
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use xayn_test_utils::error::Panic;

use crate::{
    mind::{
        config::{GridSearchConfig, PersonaBasedConfig, SaturationConfig, StateConfig},
        data::{read, DocumentProvider, Impression, Ndcg, SpecificTopics, Users},
        state::{SaturationIteration, SaturationResult, SaturationTopicResult, State},
    },
    models::UserId,
    personalization::routes::PersonalizeBy,
    storage::memory::Storage,
};

/// Runs the persona-based mind benchmark.
#[tokio::test]
#[ignore = "run on demand via `just mind-benchmark persona`"]
async fn run_persona_benchmark() -> Result<(), Panic> {
    let users_interests = Users::new("user_categories.json")?;
    let document_provider = DocumentProvider::new("news.tsv")?;
    let state = State::new(Storage::default(), StateConfig::default())?;
    // load documents from document provider to state
    state.insert(document_provider.to_documents()).await?;
    let benchmark_config = PersonaBasedConfig::default();

    // create 3d array of zeros with shape (users, iterations, nranks)
    let mut ndcgs = Ndcg::new([
        users_interests.len(),
        benchmark_config.iterations,
        benchmark_config.nranks.len(),
    ]);
    let mut rng = StdRng::seed_from_u64(42);
    for (idx, (user_id, interests)) in users_interests.iter().enumerate() {
        let interesting_documents = document_provider.get_all_interest(interests);
        // prepare reranker by interacting with documents to prepare
        state
            .interact(
                user_id,
                interesting_documents
                    .choose_multiple(&mut rng, benchmark_config.amount_of_doc_used_to_prepare)
                    .map(|doc| {
                        (
                            &doc.id,
                            // TODO: set some meaningful value for the interaction time
                            state.time - Duration::days(0),
                        )
                    }),
            )
            .await?;

        for iter in 0..benchmark_config.iterations {
            let personalised_documents = state
                .personalize(
                    user_id,
                    PersonalizeBy::knn_search(benchmark_config.ndocuments),
                    state.time,
                )
                .await?
                .unwrap();

            let scores = document_provider.score(
                &personalised_documents,
                interests,
                benchmark_config.is_semi_interesting,
            );
            ndcgs.assign(s![idx, iter, ..], &scores, &benchmark_config.nranks);
            // interact with documents
            state
                .interact(
                    user_id,
                    personalised_documents
                        .iter()
                        .zip(scores.iter())
                        .filter(|(_, &score)| {
                            (score - 2.0).abs() < 0.001
                                && rng.gen_bool(benchmark_config.click_probability)
                        })
                        .map(|(id, _)| {
                            (
                                id,
                                // TODO: set some meaningful value for the interaction time
                                state.time - Duration::days(0),
                            )
                        }),
                )
                .await?;
        }
    }
    ndcgs.write(File::create("results/persona_based_benchmark_results.npy")?)?;

    Ok(())
}

/// Runs the user-based mind benchmark.
#[tokio::test]
#[ignore = "run on demand via `just mind-benchmark user`"]
async fn run_user_benchmark() -> Result<(), Panic> {
    let document_provider = DocumentProvider::new("news.tsv")?;

    let state = State::new(Storage::default(), StateConfig::default())?;
    state.insert(document_provider.to_documents()).await?;

    let nranks = vec![5, 10];
    let mut ndcgs = Ndcg::new([nranks.len(), 0]);
    let mut users = Vec::new();

    // Loop over all impressions, prepare reranker with news in click history
    // and rerank the news in an impression
    for impression in read::<Impression>("behaviors.tsv")? {
        let impression = impression?;
        let labels = if let Some(clicks) = impression.clicks {
            let user = UserId::new(&impression.user_id).unwrap();
            if !users.contains(&impression.user_id) {
                users.push(impression.user_id);
                state
                    .interact(
                        &user,
                        clicks.iter().map(|document| {
                            (
                                document,
                                // TODO: set some meaningful value for the interaction time
                                state.time - Duration::days(0),
                            )
                        }),
                    )
                    .await?;
            }

            let document_ids = impression
                .news
                .iter()
                .map(|document| &document.document_id)
                .collect::<Vec<_>>();

            state
                .personalize(
                    &user,
                    PersonalizeBy::Documents(document_ids.as_slice()),
                    state.time,
                )
                .await?
                .unwrap()
                .iter()
                .map(|reranked_id| {
                    u8::from(
                        impression.news[document_ids
                            .iter()
                            .position(|&actual_id| actual_id == reranked_id)
                            .unwrap()]
                        .was_clicked,
                    )
                    .into()
                })
                .collect_vec()
        } else {
            impression
                .news
                .iter()
                .map(|viewed_document| u8::from(viewed_document.was_clicked).into())
                .collect_vec()
        };
        ndcgs.push(&labels, &nranks)?;
    }
    println!("{ndcgs:?}");

    Ok(())
}

/// The function panics if the provided filenames are not correct.
#[tokio::test]
#[ignore = "run on demand via `just mind-benchmark saturation`"]
async fn run_saturation_benchmark() -> Result<(), Panic> {
    // load list of possible specific topics from file (need to create it)
    let specific_topics = SpecificTopics::new("topics.json")?;
    let document_provider = DocumentProvider::new("news.tsv")?;
    let state = State::new(Storage::default(), StateConfig::default())?;
    // load documents from document provider to state
    state.insert(document_provider.to_documents()).await?;
    let benchmark_config = SaturationConfig::default();

    // create rng thread for random number generation with seed 42
    let mut rng = StdRng::seed_from_u64(42);

    // create results preserving structure
    let mut results = SaturationResult::new(specific_topics.len());

    // iterate over specific topics get the documents for each topic and interact with one document
    // from each topic and then run iterations of personalized search
    for full_category in specific_topics.iter() {
        //create saturation topic result structure
        let mut topic_result =
            SaturationTopicResult::new(full_category, benchmark_config.iterations);

        let documents = document_provider.get_all_interest(slice::from_ref(full_category));
        let user_id = UserId::new(full_category)?;
        // get random document from the topic
        let document = documents.choose(&mut rng).unwrap();

        // interact with the document
        state
            .interact(&user_id, [(&document.id, state.time - Duration::days(0))])
            .await?;

        for _ in 0..benchmark_config.iterations {
            let personalised_documents = state
                .personalize(
                    &user_id,
                    PersonalizeBy::knn_search(benchmark_config.ndocuments),
                    state.time,
                )
                .await?
                .unwrap();
            // calculate scores for the documents
            let scores = document_provider.score(
                &personalised_documents,
                slice::from_ref(full_category),
                false,
            );
            let to_be_clicked = personalised_documents
                .iter()
                .zip(scores)
                .filter_map(|(id, score)| {
                    ((score - 2.0).abs() < 0.001
                        && rng.gen_bool(benchmark_config.click_probability))
                    .then(|| id.clone())
                })
                .collect_vec();
            // interact with documents
            state
                .interact(
                    &user_id,
                    to_be_clicked
                        .iter()
                        .map(|id| (id, state.time - Duration::days(0))),
                )
                .await?;

            // add results to the topic result
            topic_result.push(SaturationIteration {
                shown_documents: personalised_documents,
                clicked_documents: to_be_clicked,
            });
        }
        results.push(topic_result);
    }
    //save results to json file
    let file = File::create("saturation_results.json")?;
    serde_json::to_writer(file, &results)?;

    Ok(())
}

#[tokio::test]
#[ignore = "run on demand via `just mind-benchmark persona_hot_news`"]
async fn run_persona_hot_news_benchmark() -> Result<(), Panic> {
    let users_interests = Users::new("user_categories.json")?;
    let document_provider = DocumentProvider::new("news.tsv")?;
    let state = State::new(Storage::default(), StateConfig::default())?;
    // load documents from document provider to state
    state.insert(document_provider.to_documents()).await?;
    let benchmark_config = PersonaBasedConfig::default();

    // create 3d array of zeros with shape (users, iterations, nranks)
    let mut ndcgs = Ndcg::new([
        users_interests.len(),
        benchmark_config.iterations,
        benchmark_config.nranks.len(),
    ]);
    let mut rng = StdRng::seed_from_u64(42);
    for (idx, (user_id, interests)) in users_interests.iter().enumerate() {
        let interesting_documents = document_provider.get_all_interest(interests);
        // prepare reranker by interacting with documents to prepare
        state
            .interact(
                user_id,
                interesting_documents
                    .choose_multiple(&mut rng, benchmark_config.amount_of_doc_used_to_prepare)
                    .map(|doc| (&doc.id, state.time - Duration::days(0))),
            )
            .await?;

        for iter in 0..benchmark_config.iterations {
            // get random news that will represent tenant's hot news category
            let hot_news = document_provider.sample(benchmark_config.ndocuments_hot_news); // number of news will be configurable

            // go through hot news and if they are in user's sphere of interests then click them with some probability
            state
                .interact(
                    user_id,
                    hot_news.iter().filter_map(|doc| {
                        (doc.is_interesting(interests)
                            && rng.gen_bool(benchmark_config.click_probability))
                        .then(|| (&doc.id, state.time - Duration::days(0)))
                    }),
                )
                .await?;

            let personalised_documents = state
                .personalize(
                    user_id,
                    PersonalizeBy::knn_search(benchmark_config.ndocuments),
                    state.time,
                )
                .await?
                .unwrap();

            let scores = document_provider.score(
                &personalised_documents,
                interests,
                benchmark_config.is_semi_interesting,
            );
            ndcgs.assign(s![idx, iter, ..], &scores, &benchmark_config.nranks);
            // interact with documents
            state
                .interact(
                    user_id,
                    personalised_documents
                        .iter()
                        .zip(scores)
                        .filter_map(|(id, score)| {
                            ((score - 2.0).abs() < 0.001
                                && rng.gen_bool(benchmark_config.click_probability))
                            .then(|| (id, state.time - Duration::days(0)))
                        }),
                )
                .await?;
        }
    }
    ndcgs.write(File::create("results/persona_based_benchmark_results.npy")?)?;

    Ok(())
}

/// Grid search for best parameters for persona based benchmark.
#[tokio::test]
#[ignore = "run on demand"]
async fn grid_search_for_best_parameters() -> Result<(), Panic> {
    // load users interests sample as computing all users interests is too expensive in grid search
    let users_interests = Users::new("user_categories_sample.json")?;
    let document_provider = DocumentProvider::new("news.tsv")?;
    let grid_search_config = GridSearchConfig::default();
    let configs = grid_search_config.create_state_configs()?;
    let mut state = State::new(Storage::default(), StateConfig::default())?;
    state.insert(document_provider.to_documents()).await?;
    let mut rng = StdRng::seed_from_u64(42);
    let file = File::create("params.json")?;
    let mut writer = io::BufWriter::new(file);
    serde_json::to_writer(&mut writer, &configs)?;
    writer.flush()?;

    let mut ndcgs = Ndcg::new([
        configs.len(),
        users_interests.len(),
        grid_search_config.iterations,
        grid_search_config.nranks.len(),
    ]);
    for (config_idx, config) in configs.into_iter().enumerate() {
        state.with_coi_config(config.coi);
        for (idx, (user_id, interests)) in users_interests.iter().enumerate() {
            let interesting_documents = document_provider.get_all_interest(interests);
            state
                .interact(
                    user_id,
                    interesting_documents
                        .choose_multiple(&mut rng, state.coi.config().min_cois())
                        .map(|doc| (&doc.id, state.time - Duration::days(0))),
                )
                .await?;

            for iter in 0..grid_search_config.iterations {
                let personalised_documents = state
                    .personalize(
                        user_id,
                        PersonalizeBy::knn_search(grid_search_config.ndocuments),
                        state.time,
                    )
                    .await?
                    .unwrap();

                let scores = document_provider.score(
                    &personalised_documents,
                    interests,
                    grid_search_config.is_semi_interesting,
                );
                ndcgs.assign(
                    s![config_idx, idx, iter, ..],
                    &scores,
                    &grid_search_config.nranks,
                );
                // interact with documents
                state
                    .interact(
                        user_id,
                        personalised_documents
                            .iter()
                            .zip(scores)
                            .filter_map(|(id, score)| {
                                ((score - 2.0).abs() < 0.001
                                    && rng.gen_bool(grid_search_config.click_probability))
                                .then_some((id, state.time))
                            }),
                    )
                    .await?;
            }
        }
    }

    Ok(())
}
