// Copyright 2023 Xayn AG
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

use actix_web::{
    web::{Data, Json, Path},
    HttpResponse,
    Responder,
};
use chrono::Utc;
use itertools::Itertools;
use serde::Deserialize;

use crate::{
    app::{AppState, TenantState},
    frontoffice::shared::{update_interactions, UnvalidatedSnippetOrDocumentId},
    models::SnippetOrDocumentId,
    Error,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnvalidatedUserInteraction {
    id: UnvalidatedSnippetOrDocumentId,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UnvalidatedUserInteractionRequest {
    documents: Vec<UnvalidatedUserInteraction>,
}

impl UnvalidatedUserInteractionRequest {
    fn validate(self) -> Result<Vec<SnippetOrDocumentId>, Error> {
        self.documents
            .into_iter()
            .map(|document| document.id.validate())
            .try_collect()
    }
}

pub(super) async fn interactions(
    state: Data<AppState>,
    user_id: Path<String>,
    Json(body): Json<UnvalidatedUserInteractionRequest>,
    TenantState(storage, _): TenantState,
) -> Result<impl Responder, Error> {
    let user_id = user_id.into_inner().try_into()?;
    let interactions = body.validate()?;
    update_interactions(
        &storage,
        &state.coi,
        &user_id,
        interactions,
        state.config.personalization.store_user_history,
        Utc::now(),
    )
    .await?;

    Ok(HttpResponse::NoContent())
}
