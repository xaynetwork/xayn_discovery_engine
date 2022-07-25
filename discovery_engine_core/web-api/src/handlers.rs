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

use std::convert::Infallible;
use warp::hyper::StatusCode;
use xayn_discovery_engine_ai::CoiSystemState;
use xayn_discovery_engine_providers::Market;

use crate::{
    db::Db,
    models::{Article, InteractionRequestBody, UserId},
};

pub(crate) async fn handle_ranked_documents(
    user_id: UserId,
    db: Db,
) -> Result<impl warp::Reply, Infallible> {
    let default_state = CoiSystemState::default();
    let user_interests = db.user_interests.read().await;
    let state = user_interests.get(&user_id).unwrap_or(&default_state);

    let mut documents = db.documents.clone();

    db.coi.rank(&mut documents, &state.user_interests);

    let articles = documents
        .into_iter()
        .map(|doc| doc.article)
        .collect::<Vec<Article>>();

    Ok(warp::reply::json(&articles))
}

pub(crate) async fn handle_user_interaction(
    user_id: UserId,
    body: InteractionRequestBody,
    db: Db,
) -> Result<impl warp::Reply, Infallible> {
    if let Some(document) = db.documents_by_id.get(&body.document_id) {
        let mut user_interests = db.user_interests.write().await;
        let state = user_interests.entry(user_id).or_default();

        db.coi.log_positive_user_reaction(
            &mut state.user_interests.positive,
            &Market::new("en", "US"),
            &mut state.key_phrases,
            &document.smbert_embedding,
            &[], //candidates,
            |words| db.smbert.run(words).map_err(Into::into),
        );

        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

pub(crate) async fn handle_clean_state(db: Db) -> Result<impl warp::Reply, Infallible> {
    db.user_interests.write().await.clear();
    Ok(StatusCode::OK)
}
