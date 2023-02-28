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

use std::{fs::File, io, io::Write};

use anyhow::Error;
use chrono::Duration;
use itertools::Itertools;
use ndarray::{Array, Array3, Array4, ArrayView};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};

use crate::{
    mind::{
        config::{
            create_grid_search_configs,
            GridSearchConfig,
            PersonaBasedConfig,
            SaturationConfig,
            StateConfig,
        },
        data::{
            read,
            score_documents,
            write_array,
            DocumentProvider,
            Impression,
            SpecificTopics,
            Users,
        },
        state::{ndcg, SaturationIteration, SaturationResult, SaturationTopicResult, State},
    },
    models::UserId,
    personalization::routes::PersonalizeBy,
    storage::memory::Storage,
};

/// Runs the persona-based mind benchmark.
#[tokio::test]
#[ignore = "run on demand via `just mind-benchmark persona`"]
async fn run_persona_benchmark() -> Result<(), Error> {
    let users_interests = Users::new("user_categories.json")?;
    let document_provider = DocumentProvider::new("news.tsv")?;

    let state = State::new(Storage::default(), StateConfig::default()).unwrap();
    // load documents from document provider to state
    state
        .insert(document_provider.documents.values().cloned().collect_vec())
        .await
        .unwrap();
    let benchmark_config = PersonaBasedConfig::default();
    // create 3d array of zeros with shape (users, iterations, nranks)
    let mut results = Array3::<f32>::zeros([
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
            .await
            .unwrap();

        for iter in 0..benchmark_config.iterations {
            let personalised_documents = state
                .personalize(
                    user_id,
                    PersonalizeBy::KnnSearch {
                        count: benchmark_config.ndocuments,
                        published_after: None,
                    },
                    state.time,
                )
                .await
                .unwrap()
                .unwrap();

            let documents = personalised_documents
                .iter()
                .map(|id| document_provider.get(id).unwrap())
                .collect_vec();

            let scores =
                score_documents(&documents, interests, benchmark_config.is_semi_interesting);
            let ndcgs_iteration = ndcg(&scores, &benchmark_config.nranks);
            //save scores to results array
            for (i, ndcg) in ndcgs_iteration.iter().enumerate() {
                results[[idx, iter, i]] = *ndcg;
            }
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
                .await
                .unwrap();
        }
    }
    let mut file = File::create("results/persona_based_benchmark_results.npy")?;
    write_array(&mut file, &results)?;
    Ok(())
}

/// Runs the user-based mind benchmark.
#[tokio::test]
#[ignore = "run on demand via `just mind-benchmark user`"]
async fn run_user_benchmark() -> Result<(), Error> {
    let document_provider = DocumentProvider::new("news.tsv")?;

    let state = State::new(Storage::default(), StateConfig::default()).unwrap();
    state
        .insert(document_provider.to_documents())
        .await
        .unwrap();

    let nranks = vec![5, 10];
    let mut ndcgs = Array::zeros((nranks.len(), 0));
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
                    .await
                    .unwrap();
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
                .await
                .unwrap()
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

        let ndcgs_iteration = ndcg(&labels, &nranks);
        ndcgs
            .push_column(ArrayView::from(&ndcgs_iteration))
            .unwrap();
    }
    println!("{ndcgs:?}");

    Ok(())
}

/// The function panics if the provided filenames are not correct.
#[tokio::test]
#[ignore = "run on demand via `just mind-benchmark saturation`"]
async fn run_saturation_benchmark() -> Result<(), Error> {
    // load list of possible specific topics from file (need to create it)
    let specific_topics = SpecificTopics::new("topics.json")?;
    let document_provider = DocumentProvider::new("news.tsv")?;
    let state = State::new(Storage::default(), StateConfig::default()).unwrap();
    // load documents from document provider to state
    state
        .insert(document_provider.documents.values().cloned().collect_vec())
        .await
        .unwrap();
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

        let documents = document_provider.get_all_interest(std::slice::from_ref(full_category));
        let user_id = UserId::new(full_category).unwrap();
        // get random document from the topic
        let document = documents.choose(&mut rng).unwrap();

        // interact with the document
        state
            .interact(&user_id, [(&document.id, state.time - Duration::days(0))])
            .await
            .unwrap();

        for _ in 0..benchmark_config.iterations {
            let personalised_documents = state
                .personalize(
                    &user_id,
                    PersonalizeBy::KnnSearch {
                        count: benchmark_config.ndocuments,
                        published_after: None,
                    },
                    state.time,
                )
                .await
                .unwrap()
                .unwrap();
            // calculate scores for the documents
            let documents = personalised_documents
                .iter()
                .map(|id| document_provider.get(id).unwrap())
                .collect_vec();

            let scores = score_documents(&documents, &[full_category.clone()], false);
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
                .await
                .unwrap();

            // add results to the topic result
            topic_result.iterations.push(SaturationIteration {
                shown_documents: personalised_documents,
                clicked_documents: to_be_clicked,
            });
        }
        results.topics.push(topic_result);
    }
    //save results to json file
    let file = File::create("saturation_results.json")?;
    serde_json::to_writer(file, &results)?;

    Ok(())
}

#[tokio::test]
#[ignore = "run on demand via `just mind-benchmark persona_hot_news`"]
async fn run_persona_hot_news_benchmark() -> Result<(), Error> {
    let users_interests = Users::new("user_categories.json")?;
    let document_provider = DocumentProvider::new("news.tsv")?;
    let state = State::new(Storage::default(), StateConfig::default()).unwrap();
    // load documents from document provider to state
    state
        .insert(document_provider.documents.values().cloned().collect_vec())
        .await
        .unwrap();
    let benchmark_config = PersonaBasedConfig::default();
    // create 3d array of zeros with shape (users, iterations, nranks)
    let mut results = Array3::<f32>::zeros([
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
            .await
            .unwrap();

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
                .await
                .unwrap();

            let personalised_documents = state
                .personalize(
                    user_id,
                    PersonalizeBy::KnnSearch {
                        count: benchmark_config.ndocuments,
                        published_after: None,
                    },
                    state.time,
                )
                .await
                .unwrap()
                .unwrap();

            let documents = personalised_documents
                .iter()
                .map(|id| document_provider.get(id).unwrap())
                .collect_vec();

            let scores =
                score_documents(&documents, interests, benchmark_config.is_semi_interesting);
            let ndcgs_iteration = ndcg(&scores, &benchmark_config.nranks);
            //save scores to results array
            results
                .slice_mut(ndarray::s![idx, iter, ..])
                .assign(&Array::from(ndcgs_iteration));
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
                .await
                .unwrap();
        }
    }
    let mut file = File::create("results/persona_based_benchmark_results.npy")?;
    write_array(&mut file, &results)?;
    Ok(())
}

/// Grid search for best parameters for persona based benchmark.
#[tokio::test]
#[ignore = "run on demand"]
async fn grid_search_for_best_parameters() -> Result<(), Error> {
    // load users interests sample as computing all users interests is too expensive in grid search
    let users_interests = Users::new("user_categories_sample.json")?;
    let document_provider = DocumentProvider::new("news.tsv")?;
    let grid_search_config = GridSearchConfig::default();
    let configs = create_grid_search_configs(&grid_search_config);
    let mut state = State::new(Storage::default(), StateConfig::default()).unwrap();
    state
        .insert(document_provider.to_documents())
        .await
        .unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let file = File::create("params.json")?;
    let mut writer = io::BufWriter::new(file);
    serde_json::to_writer(&mut writer, &configs)?;
    writer.flush()?;

    let mut ndcgs_all_configs = Array4::zeros((
        configs.len(),
        users_interests.len(),
        grid_search_config.iterations,
        grid_search_config.nranks.len(),
    ));
    for (config_idx, config) in configs.into_iter().enumerate() {
        state.with_coi_config(config.coi);
        for (idx, (user_id, interests)) in users_interests.iter().enumerate() {
            let interesting_documents = document_provider.get_all_interest(interests);
            state
                .interact(
                    user_id,
                    interesting_documents
                        .choose_multiple(&mut rng, state.coi.config().min_positive_cois())
                        .map(|doc| (&doc.id, state.time - Duration::days(0))),
                )
                .await
                .unwrap();

            for iter in 0..grid_search_config.iterations {
                let personalised_documents = state
                    .personalize(
                        user_id,
                        PersonalizeBy::KnnSearch {
                            count: grid_search_config.ndocuments,
                            published_after: None,
                        },
                        state.time,
                    )
                    .await
                    .unwrap()
                    .unwrap();

                let documents = personalised_documents
                    .iter()
                    .map(|id| document_provider.get(id).unwrap())
                    .collect_vec();

                let scores = score_documents(
                    &documents,
                    interests,
                    grid_search_config.is_semi_interesting,
                );
                let ndcgs_iteration = ndcg(&scores, &grid_search_config.nranks);
                //save scores to results array
                ndcgs_all_configs
                    .slice_mut(ndarray::s![config_idx, idx, iter, ..])
                    .assign(&Array::from(ndcgs_iteration));
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
                    .await
                    .unwrap();
            }
        }
    }
    Ok(())
}
