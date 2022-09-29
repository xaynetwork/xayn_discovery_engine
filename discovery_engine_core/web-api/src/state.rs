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

use std::sync::Arc;
use xayn_discovery_engine_ai::{CoiConfig, CoiSystem, GenericError};

use crate::{elastic, elastic::ElasticState, storage::UserState};

#[derive(Clone, Debug)]
pub struct InitConfig {
    /// PostgreSQL url
    pub pg_url: String,
    /// Elastic configuration.
    pub elastic: elastic::Config,
    /// Max nb of Positive CoIs to use in knn search.
    pub max_cois_for_knn: usize,
    /// Max nb of documents to return using personalized documents endpoint.
    pub default_documents_count: usize,
}

pub struct AppState {
    /// The center of interest (coi) system.
    pub(crate) coi: CoiSystem,
    /// Elastic Search client.
    pub(crate) elastic: ElasticState,
    /// Handler for storing the user state.
    pub(crate) user: UserState,
    /// Max nb of Positive CoIs to use in knn search.
    pub(crate) max_cois_for_knn: usize,
    /// Max nb of documents to return using personalized documents endpoint.
    pub(crate) default_documents_count: usize,
}

impl AppState {
    pub async fn init(config: InitConfig) -> Result<Arc<Self>, GenericError> {
        let user = UserState::connect(&config.pg_url).await?;
        user.init_database().await?;

        let coi = CoiConfig::default().build();
        let elastic = ElasticState::new(config.elastic);
        let app_state = AppState {
            coi,
            elastic,
            user,
            max_cois_for_knn: config.max_cois_for_knn,
            default_documents_count: config.default_documents_count,
        };

        Ok(Arc::new(app_state))
    }
}
