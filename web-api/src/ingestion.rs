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

mod routes;

use actix_web::web::ServiceConfig;
use async_trait::async_trait;
use derive_more::AsRef;
use serde::{Deserialize, Serialize};

use crate::{
    app::{self, Application, SetupError},
    embedding,
    logging,
    net,
    storage,
    tenants,
};

pub struct Ingestion;

#[async_trait]
impl Application for Ingestion {
    const NAME: &'static str = "XAYN_INGESTION";

    type Config = Config;
    type Extension = Extension;

    fn configure_service(config: &mut ServiceConfig) {
        routes::configure_service(config);
    }

    fn configure_ops_service(config: &mut ServiceConfig) {
        routes::configure_ops_service(config);
    }

    fn create_extension(_config: &Self::Config) -> Result<Self::Extension, SetupError> {
        Ok(Extension {})
    }
}

type AppState = app::AppState<Ingestion>;

#[derive(AsRef, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub(crate) logging: logging::Config,
    pub(crate) net: net::Config,
    pub(crate) storage: storage::Config,
    pub(crate) ingestion: IngestionConfig,
    pub(crate) embedding: embedding::Config,
    pub(crate) tenants: tenants::Config,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct IngestionConfig {
    pub(crate) max_document_batch_size: usize,
    pub(crate) max_indexed_properties: usize,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            max_document_batch_size: 100,
            // 10 + publication_date
            max_indexed_properties: 11,
        }
    }
}

#[derive(AsRef)]
pub struct Extension {}
