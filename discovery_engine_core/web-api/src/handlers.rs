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

use futures::future::join_all;
use itertools::Itertools;
use std::{sync::Arc, time::Duration};
use warp::{hyper::StatusCode, reject::Reject, Rejection};

use xayn_discovery_engine_ai::{
    compute_coi_relevances,
    nan_safe_f32_cmp,
    system_time_now,
    utils::rank,
    CoiContextError,
    GenericError,
    PositiveCoi,
};

use crate::{
    elastic::KnnSearchParams,
    models::{Error, InteractionRequestBody, UserId},
    state::AppState,
};

pub(crate) async fn handle_ranked_documents(
    user_id: UserId,
    state: Arc<AppState>,
) -> Result<impl warp::Reply, Rejection> {
    let user_interests = state
        .user
        .fetch_interests(&user_id)
        .await
        .map_err(handle_user_state_op_error)?;

    if user_interests.positive.is_empty() {
        return Err(warp::reject::custom(NoCenterOfInterestsError));
    }

    let cois = &user_interests.positive;
    let horizon = state.coi.config().horizon();
    let coi_weights = compute_coi_weights(cois, horizon);
    let cois = cois
        .iter()
        .zip(coi_weights)
        .sorted_by(|(_, a_weight), (_, b_weight)| nan_safe_f32_cmp(b_weight, a_weight))
        .collect_vec();

    let max_cois = state.max_cois_for_knn.min(user_interests.positive.len());
    let cois = &cois[0..max_cois];
    let weights_sum: f32 = cois.iter().map(|(_, w)| w).sum();

    let excluded = &state
        .user
        .fetch_interacted_document_ids(&user_id)
        .await
        .map_err(handle_user_state_op_error)?;
    let max_documents_count = state.max_documents_count;
    let document_futures = cois
        .iter()
        .map(|(coi, weight)| async {
            // weights_sum can't be zero, because coi weights will always return some weights that are > 0
            let weight = *weight / weights_sum;
            #[allow(
                // fine as max documents count is small enough
                clippy::cast_precision_loss,
                // fine as weight should be between 0 and 1
                clippy::cast_sign_loss,
                // fine as number of neighbors is small enough
                clippy::cast_possible_truncation
            )]
            let k_neighbors = (weight * max_documents_count as f32).ceil() as usize;

            state
                .elastic
                .get_documents_by_embedding(KnnSearchParams {
                    excluded: excluded.clone(),
                    embedding: coi.point.to_vec(),
                    size: k_neighbors,
                    k_neighbors,
                    num_candidates: max_documents_count,
                })
                .await
        })
        .collect_vec();

    let mut all_documents = Vec::new();
    let mut errors = Vec::new();

    for results in join_all(document_futures).await {
        match results {
            Ok(documents) => all_documents.extend(documents),
            Err(error) => {
                // TODO TO-3294: Add tracing
                // error!("Error fetching document: {error}");
                errors.push(error);
            }
        };
    }

    if all_documents.is_empty() && !errors.is_empty() {
        return Err(warp::reject::custom(ElasticOpError));
    }

    let scores = state
        .coi
        .score(&all_documents, &user_interests)
        // TODO TO-3339: Return 500 with the correct kind if this fail
        .map_err(handle_ranking_error)?;
    rank(&mut all_documents, &scores);

    let max_docs = max_documents_count.min(all_documents.len());
    let documents = &all_documents[0..max_docs];

    Ok(warp::reply::json(&documents))
}

pub(crate) async fn handle_user_interaction(
    user_id: UserId,
    body: InteractionRequestBody,
    state: Arc<AppState>,
) -> Result<impl warp::Reply, Rejection> {
    let documents = state
        .elastic
        .get_documents_by_ids(&[body.document_id])
        .await
        .map_err(handle_elastic_error)?;

    if let Some(document) = documents.first() {
        state
            .user
            .update_positive_cois(&document.id, &user_id, |positive_cois| {
                state
                    .coi
                    .log_positive_user_reaction(positive_cois, &document.embedding)
            })
            .await
            .map_err(handle_user_state_op_error)?;

        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

/// Computes [`PositiveCoi`]s weights used to determine how many documents to fetch using each centers' embedding.
fn compute_coi_weights(cois: &[PositiveCoi], horizon: Duration) -> Vec<f32> {
    let relevances = compute_coi_relevances(cois, horizon, system_time_now())
        .into_iter()
        .map(|rel| 1.0 - (-3.0 * rel).exp())
        .collect_vec();

    let rel_sum: f32 = relevances.iter().sum();
    relevances
        .iter()
        .map(|rel| {
            let res = rel / rel_sum;
            if res.is_nan() {
                // should be ok for our use-case
                #[allow(clippy::cast_precision_loss)]
                let len = cois.len() as f32;
                // len can't be zero, because we return early if we have no positive CoIs and never compute weights
                1.0f32 / len
            } else {
                res
            }
        })
        .collect()
}

fn handle_user_state_op_error(_: GenericError) -> Rejection {
    warp::reject::custom(UserStateOpError)
}

fn handle_elastic_error(_: Error) -> Rejection {
    warp::reject::custom(ElasticOpError)
}

fn handle_ranking_error(_: CoiContextError) -> Rejection {
    warp::reject::custom(RankingError)
}

#[derive(Debug)]
struct UserStateOpError;
impl Reject for UserStateOpError {}

#[derive(Debug)]
struct ElasticOpError;
impl Reject for ElasticOpError {}

#[derive(Debug)]
struct NoCenterOfInterestsError;
impl Reject for NoCenterOfInterestsError {}

#[derive(Debug)]
struct RankingError;
impl Reject for RankingError {}
