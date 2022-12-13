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

pub(crate) mod routes;

use actix_web::web::ServiceConfig;
use derive_more::AsRef;
use serde::{Deserialize, Serialize};
use xayn_ai_coi::{CoiConfig, CoiSystem};

use crate::{
    server::{self, Application},
    storage::Storage,
};

pub struct Personalization;

impl Application for Personalization {
    type ConfigExtension = ConfigExtension;
    type AppStateExtension = AppStateExtension;

    fn configure_service(config: &mut ServiceConfig) {
        routes::configure_service(config);
    }

    fn create_app_state_extension(
        config: &server::Config<Self::ConfigExtension>,
    ) -> Result<Self::AppStateExtension, server::SetupError> {
        Ok(AppStateExtension {
            coi: config.extension.coi.clone().build(),
        })
    }
}

type AppState = server::AppState<
    <Personalization as Application>::ConfigExtension,
    <Personalization as Application>::AppStateExtension,
    Storage,
>;

#[derive(AsRef, Debug, Default, Deserialize, Serialize)]
pub struct ConfigExtension {
    #[as_ref]
    #[serde(default)]
    pub(crate) coi: CoiConfig,

    #[as_ref]
    #[serde(default)]
    pub(crate) personalization: PersonalizationConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct PersonalizationConfig {
    /// Max number of documents to return.
    #[serde(default = "default_max_number_documents")]
    pub(crate) max_number_documents: usize,
    /// Default number of documents to return.
    #[serde(default = "default_default_number_documents")]
    pub(crate) default_number_documents: usize,
    /// Max number of positive cois to use in knn search.
    #[serde(default = "default_max_cois_for_knn")]
    pub(crate) max_cois_for_knn: usize,
    /// Weighting of user interests vs document categories. Must be in the interval `[0, 1]`.
    #[serde(default = "default_interest_category_bias")]
    pub(crate) interest_category_bias: f32,
}

fn default_max_number_documents() -> usize {
    100
}

fn default_default_number_documents() -> usize {
    100
}

fn default_max_cois_for_knn() -> usize {
    //FIXME what is a default value we know works well with how we do knn?
    10
}

fn default_interest_category_bias() -> f32 {
    0.5
}

impl Default for PersonalizationConfig {
    fn default() -> Self {
        Self {
            max_number_documents: default_max_number_documents(),
            default_number_documents: default_default_number_documents(),
            max_cois_for_knn: default_max_cois_for_knn(),
            interest_category_bias: default_interest_category_bias(),
        }
    }
}

#[derive(AsRef)]
pub struct AppStateExtension {
    #[as_ref]
    pub(crate) coi: CoiSystem,
}
