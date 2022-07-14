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

use uuid::Uuid;
use warp::hyper::StatusCode;

use crate::{db::Db, models::UserInteractionDto};

// TODO: TY-3013
#[allow(clippy::unused_async)]
pub(crate) async fn handle_ranked_documents(
    _user_id: Uuid,
    _db: Db,
) -> Result<impl warp::Reply, Infallible> {
    Ok(StatusCode::NOT_IMPLEMENTED)
}

// TODO: TY-3014
#[allow(clippy::unused_async)]
pub(crate) async fn handle_user_interaction(
    _user_id: Uuid,
    _body: UserInteractionDto,
    _db: Db,
) -> Result<impl warp::Reply, Infallible> {
    Ok(StatusCode::NOT_IMPLEMENTED)
}

// TODO: TY-3015
#[allow(clippy::unused_async)]
pub(crate) async fn handle_clean_state(_db: Db) -> Result<impl warp::Reply, Infallible> {
    Ok(StatusCode::NOT_IMPLEMENTED)
}
