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

use actix_web::{
    web::{self, Data, Json, ServiceConfig},
    Responder,
};
use serde::Deserialize;

use crate::{
    error::application::{Unimplemented, WithRequestIdExt},
    Error,
};

use super::Config;

pub(super) fn configure_service(config: &mut ServiceConfig) {
    let resource =
        web::resource("/documents").route(web::post().to(new_documents.error_with_request_id()));

    config.service(resource);
}

//FIXME use actual body
#[derive(Deserialize)]
struct NewDocuments {}

async fn new_documents(
    _config: Data<Config>,
    _new_documents: Json<NewDocuments>,
) -> Result<impl Responder, Error> {
    if true {
        Err(Unimplemented {
            functionality: "endpoint /documents",
        })?;
    }
    Ok("text body response")
}
