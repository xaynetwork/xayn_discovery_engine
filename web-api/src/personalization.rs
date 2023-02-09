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

mod knn;
mod rerank;
pub(crate) mod routes;

use actix_web::web::ServiceConfig;
use async_trait::async_trait;
use derive_more::AsRef;
use serde::{Deserialize, Serialize};
use xayn_ai_coi::{CoiConfig, CoiSystem};

use crate::{
    app::{self, Application, SetupError},
    logging,
    net,
    storage::{self, Storage},
};

pub struct Personalization;

#[async_trait]
impl Application for Personalization {
    const NAME: &'static str = "XAYN_PERSONALIZATION";

    type Config = Config;
    type Extension = Extension;
    type Storage = Storage;

    fn configure_service(config: &mut ServiceConfig) {
        routes::configure_service(config);
    }

    fn create_extension(config: &Self::Config) -> Result<Self::Extension, SetupError> {
        Ok(Extension {
            coi: config.coi.clone().build(),
        })
    }

    async fn setup_storage(config: &storage::Config) -> Result<Self::Storage, SetupError> {
        config.setup().await
    }

    async fn close_storage(storage: &Self::Storage) {
        storage.close().await;
    }
}

type AppState = app::AppState<Personalization>;

#[derive(AsRef, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub(crate) logging: logging::Config,
    pub(crate) net: net::Config,
    pub(crate) storage: storage::Config,
    pub(crate) coi: CoiConfig,
    pub(crate) personalization: PersonalizationConfig,
    pub(crate) semantic_search: SemanticSearchConfig,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct PersonalizationConfig {
    /// Max number of documents to return.
    pub(crate) max_number_documents: usize,

    /// Default number of documents to return.
    pub(crate) default_number_documents: usize,

    /// Max number of positive cois to use in knn search.
    pub(crate) max_cois_for_knn: usize,

    /// Weighting of user interests vs document tags. Must be in the interval `[0, 1]`.
    pub(crate) interest_tag_bias: f32,

    /// Whether to store the history of user interactions.
    pub(crate) store_user_history: bool,

    /// The maximal size the history of the stateless personalization endpoint can have.
    pub(crate) max_stateless_history_size: usize,
}

impl Default for PersonalizationConfig {
    fn default() -> Self {
        Self {
            max_number_documents: 100,
            default_number_documents: 10,
            // FIXME: what is a default value we know works well with how we do knn?
            max_cois_for_knn: 10,
            interest_tag_bias: 0.5,
            store_user_history: true,
            max_stateless_history_size: 20,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct SemanticSearchConfig {
    /// Max number of documents to return.
    pub(crate) max_number_documents: usize,
    /// Default number of documents to return.
    pub(crate) default_number_documents: usize,
}

impl Default for SemanticSearchConfig {
    fn default() -> Self {
        Self {
            max_number_documents: 100,
            default_number_documents: 10,
        }
    }
}
#[derive(AsRef)]
pub struct Extension {
    pub(crate) coi: CoiSystem,
}
