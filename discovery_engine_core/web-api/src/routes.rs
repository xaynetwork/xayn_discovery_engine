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
use warp::{self, Filter, Rejection, Reply};

use crate::{db::Db, handlers, models::UserId};

pub(crate) fn api_routes(db: Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    get_ranked_documents(db.clone()).or(post_user_interaction(db))
}

// GET /user/:user_id/documents
fn get_ranked_documents(db: Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    user_path()
        .and(warp::path("documents"))
        .and(warp::get())
        .and(with_db(db))
        .and_then(handlers::handle_ranked_documents)
}

// POST /user/:user_id/interaction
fn post_user_interaction(db: Db) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    user_path()
        .and(warp::path("interaction"))
        .and(warp::post())
        .and(warp::body::content_length_limit(1024))
        .and(warp::body::json())
        .and(with_db(db))
        .and_then(handlers::handle_user_interaction)
}

// PATH /user/:user_id
fn user_path() -> impl Filter<Extract = (UserId,), Error = Rejection> + Clone {
    warp::path("user").and(warp::path::param::<UserId>())
}

fn with_db(db: Db) -> impl Filter<Extract = (Db,), Error = Infallible> + Clone {
    warp::any().map(move || db.clone())
}
