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

mod routes;

use actix_web::web::ServiceConfig;
use derive_more::AsRef;
use serde::{Deserialize, Serialize};
use xayn_discovery_engine_ai::{CoiConfig, CoiSystem};

use crate::server::{self, Application};

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
>;

#[derive(AsRef, Debug, Default, Deserialize, Serialize)]
pub struct ConfigExtension {
    #[allow(dead_code)]
    #[as_ref]
    #[serde(default)]
    pub(crate) coi: CoiConfig,

    #[as_ref]
    #[serde(default)]
    pub(crate) personalization: PersonalizationConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct PersonalizationConfig {
    #[serde(default = "default_max_number_documents")]
    pub(crate) max_number_documents: usize,
    #[serde(default = "default_default_number_documents")]
    pub(crate) default_number_documents: usize,
    #[serde(default = "default_max_cois_for_knn")]
    pub(crate) max_cois_for_knn: usize,
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

impl Default for PersonalizationConfig {
    fn default() -> Self {
        Self {
            max_number_documents: default_max_number_documents(),
            default_number_documents: default_default_number_documents(),
            max_cois_for_knn: default_max_cois_for_knn(),
        }
    }
}

#[derive(AsRef)]
pub struct AppStateExtension {
    #[as_ref]
    #[allow(dead_code)]
    pub(crate) coi: CoiSystem,
}
