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

use std::{cmp::Ordering, collections::HashMap, time::Duration};

use actix_web::{
    http::StatusCode,
    web::{self, Data, Json, Path, Query, ServiceConfig},
    Either,
    HttpResponse,
    Responder,
};
use chrono::{DateTime, Utc};
use futures_util::{stream::FuturesUnordered, StreamExt};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tracing::error;
use xayn_ai_coi::{
    compute_coi_relevances,
    nan_safe_f32_cmp,
    system_time_now,
    CoiSystem,
    PositiveCoi,
};

use super::{AppState, PersonalizationConfig, SemanticSearchConfig};
use crate::{
    error::{
        application::WithRequestIdExt,
        common::{BadRequest, DocumentNotFound, InternalError},
    },
    models::{DocumentId, DocumentProperties, PersonalizedDocument, UserId, UserInteractionType},
    storage::{self, KnnSearchParams},
    Error,
};

pub(super) fn configure_service(config: &mut ServiceConfig) {
    let users = web::scope("/users/{user_id}")
        .service(
            web::resource("interactions")
                .route(web::patch().to(interactions.error_with_request_id())),
        )
        .service(
            web::resource("personalized_documents")
                .route(web::get().to(personalized_documents.error_with_request_id())),
        );

    let semantic_search = web::resource("/semantic_search/{document_id}")
        .route(web::get().to(semantic_search.error_with_request_id()));

    config.service(users).service(semantic_search);
}

/// Represents user interaction request body.
#[derive(Clone, Debug, Deserialize)]
struct UpdateInteractions {
    documents: Vec<UserInteractionData>,
}

#[derive(Clone, Debug, Deserialize)]
struct UserInteractionData {
    #[serde(rename = "id")]
    pub(crate) document_id: String,
    #[serde(rename = "type")]
    pub(crate) interaction_type: UserInteractionType,
}

async fn interactions(
    state: Data<AppState>,
    user_id: Path<String>,
    Json(interactions): Json<UpdateInteractions>,
) -> Result<impl Responder, Error> {
    let user_id = user_id.into_inner().try_into()?;
    let interactions = interactions
        .documents
        .into_iter()
        .map(|data| {
            data.document_id
                .try_into()
                .map(|document_id| (document_id, data.interaction_type))
        })
        .try_collect::<_, Vec<_>, _>()?;
    update_interactions(
        &state.storage,
        &state.coi,
        &user_id,
        &interactions,
        state.config.personalization.store_user_history,
    )
    .await?;

    Ok(HttpResponse::NoContent())
}

pub(crate) async fn update_interactions(
    storage: &(impl storage::Document + storage::Interaction + storage::Interest + storage::Tag),
    coi: &CoiSystem,
    user_id: &UserId,
    interactions: &[(DocumentId, UserInteractionType)],
    store_user_history: bool,
) -> Result<(), Error> {
    storage::Interaction::user_seen(storage, user_id).await?;

    #[allow(clippy::zero_sized_map_values)]
    let document_id_to_interaction_type = interactions
        .iter()
        .map(|(document_id, interaction_type)| (document_id, interaction_type))
        .collect::<HashMap<_, _>>();

    let document_ids = interactions
        .iter()
        .map(|(document_id, _)| document_id)
        .collect_vec();
    storage::Interaction::update_interactions(
        storage,
        user_id,
        &document_ids,
        store_user_history,
        |context| {
            match document_id_to_interaction_type[&context.document.id] {
                UserInteractionType::Positive => {
                    for tag in &context.document.tags {
                        *context.tag_weight_diff
                            .get_mut(tag)
                            .unwrap(/* update_interactions assures all tags are given */) += 1;
                    }
                    coi.log_positive_user_reaction(
                        context.positive_cois,
                        &context.document.embedding,
                    )
                    .clone()
                }
            }
        },
    )
    .await?;

    Ok(())
}

/// Represents personalized documents query params.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PersonalizedDocumentsQuery {
    pub(crate) count: Option<usize>,
    pub(crate) published_after: Option<DateTime<Utc>>,
}

impl PersonalizedDocumentsQuery {
    fn document_count(&self, config: &PersonalizationConfig) -> Result<usize, Error> {
        let count = self.count.map_or(config.default_number_documents, |count| {
            count.min(config.max_number_documents)
        });

        if count > 0 {
            Ok(count)
        } else {
            Err(BadRequest::from("count has to be at least 1").into())
        }
    }
}

async fn personalized_documents(
    state: Data<AppState>,
    user_id: Path<String>,
    options: Query<PersonalizedDocumentsQuery>,
) -> Result<impl Responder, Error> {
    personalize_documents_by(
        &state.storage,
        &state.coi,
        &user_id.into_inner().try_into()?,
        &state.config.personalization,
        PersonalizeBy::KnnSearch {
            count: options.document_count(&state.config.personalization)?,
            published_after: options.published_after,
        },
    )
    .await
    .map(|documents| {
        if let Some(documents) = documents {
            Either::Left(Json(PersonalizedDocumentsResponse {
                documents: documents.into_iter().map(Into::into).collect(),
            }))
        } else {
            Either::Right((
                Json(PersonalizedDocumentsError::NotEnoughInteractions),
                StatusCode::CONFLICT,
            ))
        }
    })
}

#[derive(Debug, Serialize)]
struct PersonalizedDocumentData {
    id: DocumentId,
    score: f32,
    #[serde(skip_serializing_if = "DocumentProperties::is_empty")]
    properties: DocumentProperties,
}

impl From<PersonalizedDocument> for PersonalizedDocumentData {
    fn from(value: PersonalizedDocument) -> Self {
        Self {
            id: value.id,
            score: value.score,
            properties: value.properties,
        }
    }
}

/// Represents response from personalized documents endpoint.
#[derive(Debug, Serialize)]
struct PersonalizedDocumentsResponse {
    /// A list of documents personalized for a specific user.
    documents: Vec<PersonalizedDocumentData>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub(crate) enum PersonalizedDocumentsError {
    NotEnoughInteractions,
}

/// Computes [`PositiveCoi`]s weights used to determine how many documents to fetch using each center's embedding.
fn compute_coi_weights(cois: &[PositiveCoi], horizon: Duration) -> Vec<f32> {
    let relevances = compute_coi_relevances(cois, horizon, system_time_now())
        .into_iter()
        .map(|rel| 1.0 - (-3.0 * rel).exp())
        .collect_vec();

    let relevance_sum = relevances.iter().sum::<f32>();
    relevances
        .iter()
        .map(|relevance| {
            let weight = relevance / relevance_sum;
            if weight.is_finite() {
                weight
            } else {
                // should be ok for our use-case
                #[allow(clippy::cast_precision_loss)]
                let len = cois.len() as f32;
                // len can't be zero, because this iterator isn't entered for empty cois
                1. / len
            }
        })
        .collect()
}

/// Performs an approximate knn search for documents similar to the positive user interests.
#[allow(clippy::too_many_arguments)]
async fn search_knn_documents(
    storage: &(impl storage::Document + storage::Interaction),
    user_id: &UserId,
    cois: &[PositiveCoi],
    horizon: Duration,
    max_cois: usize,
    store_user_history: bool,
    count: usize,
    published_after: Option<DateTime<Utc>>,
) -> Result<Vec<PersonalizedDocument>, Error> {
    let coi_weights = compute_coi_weights(cois, horizon);
    let cois = cois
        .iter()
        .zip(coi_weights)
        .sorted_by(|(_, a_weight), (_, b_weight)| nan_safe_f32_cmp(b_weight, a_weight))
        .collect_vec();

    let cois = &cois[..max_cois.min(cois.len())];
    let weights_sum = cois.iter().map(|(_, w)| w).sum::<f32>();

    let excluded = if store_user_history {
        storage::Interaction::get(storage, user_id).await?
    } else {
        Vec::new()
    };

    let mut document_futures = cois
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
            let k_neighbors = (weight * count as f32).ceil() as usize;

            storage::Document::get_by_embedding(
                storage,
                KnnSearchParams {
                    excluded: &excluded,
                    embedding: &coi.point,
                    k_neighbors,
                    num_candidates: count,
                    published_after,
                },
            )
            .await
        })
        .collect::<FuturesUnordered<_>>();

    let mut all_documents = HashMap::new();
    let mut errors = Vec::new();

    while let Some(result) = document_futures.next().await {
        match result {
            Ok(documents) => {
                // the same document can be returned with different elastic scores, hence the
                // documents are deduplicated and only the highest score is retained for each
                for document in documents {
                    all_documents
                        .entry(document.id.clone())
                        .and_modify(|PersonalizedDocument { score, .. }| {
                            if *score < document.score {
                                *score = document.score;
                            }
                        })
                        .or_insert(document);
                }
            }
            Err(error) => {
                error!("Error fetching documents: {error}");
                errors.push(error);
            }
        };
    }

    if all_documents.is_empty() && !errors.is_empty() {
        Err(InternalError::from_message("Fetching documents failed").into())
    } else {
        Ok(all_documents.into_values().collect_vec())
    }
}

pub(crate) enum PersonalizeBy<'a> {
    KnnSearch {
        count: usize,
        published_after: Option<DateTime<Utc>>,
    },
    #[allow(dead_code)]
    Documents(&'a [&'a DocumentId]),
}

pub(crate) async fn personalize_documents_by(
    storage: &(impl storage::Document + storage::Interaction + storage::Interest + storage::Tag),
    coi: &CoiSystem,
    user_id: &UserId,
    personalization: &PersonalizationConfig,
    by: PersonalizeBy<'_>,
) -> Result<Option<Vec<PersonalizedDocument>>, Error> {
    storage::Interaction::user_seen(storage, user_id).await?;

    let cois = storage::Interest::get(storage, user_id).await?;
    if !cois.has_enough(coi.config()) {
        return Ok(None);
    }

    let mut all_documents = match by {
        PersonalizeBy::KnnSearch {
            count,
            published_after,
        } => {
            search_knn_documents(
                storage,
                user_id,
                &cois.positive,
                coi.config().horizon(),
                personalization.max_cois_for_knn,
                personalization.store_user_history,
                count,
                published_after,
            )
            .await?
        }
        PersonalizeBy::Documents(documents) => {
            storage::Document::get_personalized(storage, documents).await?
        }
    };

    coi.rank(&mut all_documents, &cois);
    let documents_by_interests = all_documents
        .iter()
        .enumerate()
        .map(|(rank, document)| (document.id.clone(), rank))
        .collect::<HashMap<_, _>>();

    let tags = storage::Tag::get(storage, user_id).await?;
    let mut documents_by_tags = all_documents
        .iter()
        .map(|document| {
            let weight = document
                .tags
                .iter()
                .map(|tag| tags.get(tag))
                .sum::<Option<usize>>()
                .unwrap_or_default();
            (document.id.clone(), weight)
        })
        .collect_vec();
    documents_by_tags.sort_unstable_by(|(_, a), (_, b)| a.cmp(b).reverse());
    let documents_by_tags = documents_by_tags
        .into_iter()
        .enumerate()
        .map(|(rank, (document_id, _))| (document_id, rank))
        .collect::<HashMap<_, _>>();

    let weight = personalization.interest_tag_bias;
    all_documents.sort_unstable_by(
        #[allow(clippy::cast_precision_loss)] // number of docs is small enough
        |a, b| {
            let weighted_a = documents_by_interests[&a.id] as f32 * weight
                + documents_by_tags[&a.id] as f32 * (1. - weight);
            let weighted_b = documents_by_interests[&b.id] as f32 * weight
                + documents_by_tags[&b.id] as f32 * (1. - weight);
            match nan_safe_f32_cmp(&weighted_a, &weighted_b) {
                Ordering::Equal if weight > 0.5 => {
                    documents_by_interests[&a.id].cmp(&documents_by_interests[&b.id])
                }
                Ordering::Equal if weight < 0.5 => {
                    documents_by_tags[&a.id].cmp(&documents_by_tags[&b.id])
                }
                ordering => ordering,
            }
        },
    );
    if let PersonalizeBy::KnnSearch { count, .. } = by {
        all_documents.truncate(count);
    }

    Ok(Some(all_documents))
}

#[derive(Deserialize)]
struct SemanticSearchQuery {
    count: Option<usize>,
}

impl SemanticSearchQuery {
    fn document_count(&self, config: &SemanticSearchConfig) -> Result<usize, Error> {
        let count = self.count.map_or(config.default_number_documents, |count| {
            count.min(config.max_number_documents)
        });

        if count > 0 {
            Ok(count)
        } else {
            Err(BadRequest::from("count has to be at least 1").into())
        }
    }
}

#[derive(Serialize)]
struct SemanticSearchResponse {
    documents: Vec<PersonalizedDocumentData>,
}

async fn semantic_search(
    state: Data<AppState>,
    document_id: Path<String>,
    query: Query<SemanticSearchQuery>,
) -> Result<impl Responder, Error> {
    let document_id = document_id.into_inner().try_into()?;
    let count = query.document_count(state.config.as_ref())?;

    let reference = storage::Document::get_personalized(&state.storage, &[&document_id])
        .await?
        .pop()
        .ok_or(DocumentNotFound)?;

    let documents = storage::Document::get_by_embedding(
        &state.storage,
        KnnSearchParams {
            excluded: &[],
            embedding: &reference.embedding,
            k_neighbors: count,
            num_candidates: count,
            published_after: None,
        },
    )
    .await?;

    Ok(Json(SemanticSearchResponse {
        documents: documents.into_iter().map(Into::into).collect(),
    }))
}

#[cfg(test)]
mod tests {
    use xayn_ai_coi::CoiConfig;

    use super::*;
    use crate::storage::memory::Storage;

    #[tokio::test]
    async fn test_search_knn_documents_for_empty_cois() {
        // these dummy values are only used for the excluded documents which will be empty
        let storage = Storage::default();
        let user = "123".try_into().unwrap();

        let documents = search_knn_documents(
            &storage,
            &user,
            &[],
            CoiConfig::default().horizon(),
            PersonalizationConfig::default().max_cois_for_knn,
            PersonalizationConfig::default().store_user_history,
            10,
            None,
        )
        .await
        .unwrap();
        assert!(documents.is_empty());
    }
}
