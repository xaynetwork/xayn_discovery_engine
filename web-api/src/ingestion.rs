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

pub(crate) mod preprocessor;
mod routes;

use actix_web::web::ServiceConfig;
use anyhow::bail;
use async_trait::async_trait;
use derive_more::AsRef;
use futures_util::TryFutureExt;
use serde::{Deserialize, Serialize};
use xayn_snippet_extractor::pool::SnippetExtractorPool;

use self::{
    preprocessor::{preprocess, PreprocessError},
    routes::InputData,
};
use crate::{
    app::{self, Application, SetupError},
    embedding,
    extractor,
    logging,
    models::{DocumentContent, PreprocessingStep},
    net,
    storage::{self, elastic::IndexUpdateConfig},
    tenants,
    Error,
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

    fn create_extension(config: &Self::Config) -> Result<Self::Extension, SetupError> {
        config.ingestion.validate()?;

        let snippet_extractor = SnippetExtractorPool::new(config.as_ref())?;

        Ok(Extension { snippet_extractor })
    }
}

type AppState = app::AppState<Ingestion>;

impl AppState {
    pub(crate) async fn preprocess(
        &self,
        original: InputData,
        preprocessing_step: &mut PreprocessingStep,
    ) -> Result<Vec<DocumentContent>, PreprocessError> {
        preprocess(
            &self.embedder,
            || self.snippet_extractor.get().map_err(Error::from),
            &self.extractor,
            original,
            preprocessing_step,
        )
        .await
    }
}

#[derive(AsRef, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct Config {
    pub(crate) logging: logging::Config,
    pub(crate) net: net::Config,
    pub(crate) storage: storage::Config,
    pub(crate) ingestion: IngestionConfig,
    pub(crate) embedding: embedding::Config,
    pub(crate) text_extractor: extractor::Config,
    pub(crate) tenants: tenants::Config,
    pub(crate) snippet_extractor: xayn_snippet_extractor::Config,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct IngestionConfig {
    pub(crate) max_document_batch_size: usize,
    pub(crate) max_indexed_properties: usize,
    pub(crate) index_update: IndexUpdateConfig,
    pub(crate) max_snippet_size: usize,
    pub(crate) max_properties_size: usize,
    pub(crate) max_properties_string_size: usize,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            max_document_batch_size: 100,
            // 10 + publication_date
            max_indexed_properties: 11,
            index_update: IndexUpdateConfig::default(),
            max_snippet_size: 2_048,
            max_properties_size: 2_560,
            max_properties_string_size: 2_048,
        }
    }
}

impl IngestionConfig {
    fn validate(&self) -> Result<(), SetupError> {
        if self.max_indexed_properties == 0 {
            bail!("invalid IngestionConfig, max_indexed_properties must be > 0 to account for publication_date");
        }
        self.index_update.validate()?;

        Ok(())
    }
}

#[derive(AsRef)]
pub struct Extension {
    snippet_extractor: SnippetExtractorPool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_default_ingestion_config() {
        IngestionConfig::default().validate().unwrap();
    }
}
