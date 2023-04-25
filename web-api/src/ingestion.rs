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
    embedding::{self, Embedder},
    logging,
    net,
    storage::{self, Storage},
    tenants,
};

pub struct Ingestion;

#[async_trait]
impl Application for Ingestion {
    const NAME: &'static str = "XAYN_INGESTION";

    type Config = Config;
    type Extension = Extension;
    type Storage = Storage;

    fn configure_service(config: &mut ServiceConfig) {
        routes::configure_service(config);
    }

    fn create_extension(config: &Self::Config) -> Result<Self::Extension, SetupError> {
        Ok(Extension {
            embedder: Embedder::load(&config.embedding)?,
        })
    }

    async fn setup_storage(config: &storage::Config) -> Result<Self::Storage, SetupError> {
        config.setup().await
    }

    async fn close_storage(storage: &Self::Storage) {
        storage.close().await;
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
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            max_document_batch_size: 100,
        }
    }
}

#[derive(AsRef)]
pub struct Extension {
    pub(crate) embedder: Embedder,
}
