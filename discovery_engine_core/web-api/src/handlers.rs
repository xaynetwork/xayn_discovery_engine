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

use warp::{hyper::StatusCode, reject::Reject, Rejection};

use xayn_discovery_engine_ai::GenericError;
use xayn_discovery_engine_providers::Market;

use crate::{
    db::Db,
    models::{Article, InteractionRequestBody, UserId},
};

pub(crate) async fn handle_ranked_documents(
    user_id: UserId,
    db: Db,
) -> Result<impl warp::Reply, Rejection> {
    let (user_interests, _) = db
        .user_state
        .fetch(&user_id)
        .await
        .map_err(handle_user_state_op_error)?
        .unwrap_or_default();

    let mut documents = db.documents.clone();

    db.coi.rank(&mut documents, &user_interests);

    let articles = documents
        .into_iter()
        .map(Article::from)
        .collect::<Vec<Article>>();

    Ok(warp::reply::json(&articles))
}

pub(crate) async fn handle_user_interaction(
    user_id: UserId,
    body: InteractionRequestBody,
    db: Db,
) -> Result<impl warp::Reply, Rejection> {
    if let Some(document) = db.documents_by_id.get(&body.document_id) {
        let (mut user_interests, mut key_phrases) = db
            .user_state
            .fetch(&user_id)
            .await
            .map_err(handle_user_state_op_error)?
            .unwrap_or_default();

        db.coi.log_positive_user_reaction(
            &mut user_interests.positive,
            &Market::new("en", "US"),
            &mut key_phrases,
            &document.smbert_embedding,
            &[], //candidates,
            |words| db.smbert.run(words).map_err(Into::into),
        );

        db.user_state
            .update(&user_id, &user_interests, &key_phrases)
            .await
            .map_err(handle_user_state_op_error)?;

        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

pub(crate) async fn handle_clean_state(db: Db) -> Result<impl warp::Reply, Rejection> {
    db.user_state
        .clear()
        .await
        .map_err(handle_user_state_op_error)?;
    Ok(StatusCode::OK)
}

fn handle_user_state_op_error(_: GenericError) -> Rejection {
    warp::reject::custom(UserStateOpError)
}

#[derive(Debug)]
struct UserStateOpError;
impl Reject for UserStateOpError {}
