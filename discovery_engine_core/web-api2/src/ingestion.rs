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
use serde::{Deserialize, Serialize};

use crate::{
    error::application::{Unimplemented, WithRequestIdExt},
    server::{self, Application},
    Error,
};

pub struct Ingestion;

impl Application for Ingestion {
    type ConfigExtension = ConfigExtension;

    fn configure(config: &mut ServiceConfig) {
        let resource = web::resource("/documents")
            .route(web::post().to(new_documents.error_with_request_id()));
        config.service(resource);
    }
}

type Config = server::Config<<Ingestion as Application>::ConfigExtension>;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ConfigExtension {
    #[allow(dead_code)]
    #[serde(default)]
    pub(crate) ingestion: IngestionConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IngestionConfig {
    #[allow(dead_code)]
    #[serde(default = "default_max_document_batch_size")]
    pub(crate) max_document_batch_size: u64,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            max_document_batch_size: default_max_document_batch_size(),
        }
    }
}

fn default_max_document_batch_size() -> u64 {
    100
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
