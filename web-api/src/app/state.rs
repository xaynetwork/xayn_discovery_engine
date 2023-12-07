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

use actix_web::{
    dev::Payload,
    web::{Data, ServiceConfig},
    FromRequest,
    HttpRequest,
};
use derive_more::AsRef;
use futures_util::future::{ready, Ready};
use xayn_ai_coi::CoiSystem;
use xayn_snippet_extractor::pool::SnippetExtractorPool;
use xayn_web_api_db_ctrl::Silo;
use xayn_web_api_shared::request::TenantId;

use crate::{
    app::SetupError,
    config::Config,
    embedding::Embedder,
    error::common::InternalError,
    extractor::TextExtractor,
    middleware::request_context::RequestContext,
    storage::{initialize_silo, Storage, StorageBuilder},
    Error,
};

#[derive(AsRef)]
pub(crate) struct AppState {
    #[as_ref(forward)]
    pub(crate) config: Config,
    pub(crate) embedder: Embedder,
    pub(crate) extractor: TextExtractor,
    pub(crate) snippet_extractor: SnippetExtractorPool,
    pub(crate) coi: CoiSystem,
    storage_builder: Arc<StorageBuilder>,
    silo: Arc<Silo>,
}

impl AppState {
    pub(super) fn attach_to(self: Arc<Self>, service: &mut ServiceConfig) {
        service
            .app_data(self.storage_builder.clone())
            .app_data(Data::from(self.silo.clone()))
            .app_data(Data::from(self));
    }

    pub(super) async fn create(config: Config) -> Result<Self, SetupError> {
        // embedder config is validated during loading
        let embedder = Embedder::load(config.as_ref()).await?;
        let extractor = TextExtractor::new(config.as_ref())?;
        let (silo, legacy_tenant) =
            initialize_silo(config.as_ref(), config.as_ref(), embedder.embedding_size()).await?;
        let storage_builder = Arc::new(Storage::builder(config.as_ref(), legacy_tenant).await?);
        let snippet_extractor = SnippetExtractorPool::new(config.as_ref())?;
        Ok(Self {
            coi: config.coi.clone().build(),
            config,
            embedder,
            extractor,
            snippet_extractor,
            storage_builder,
            silo: Arc::new(silo),
        })
    }

    pub(super) async fn close(self: Arc<Self>) {
        self.storage_builder.close().await;
    }

    pub(crate) fn legacy_tenant(&self) -> Option<&TenantId> {
        self.storage_builder.legacy_tenant()
    }
}

/// Extract tenant specific state.
///
/// For now this only extracts storage.
pub(crate) struct TenantState(pub(crate) Storage);

impl FromRequest for TenantState {
    type Error = Error;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(request: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(extract_tenant_state(request))
    }
}

fn extract_tenant_state(request: &HttpRequest) -> Result<TenantState, Error> {
    RequestContext::try_extract_from_request(request, |ctx| {
        let storage = request
            .app_data::<Arc<StorageBuilder>>()
            .ok_or_else(|| InternalError::from_message("Arc<StorageBuilder> missing"))?
            .build_for(&ctx.tenant_id);
        Ok(TenantState(storage))
    })
    .map_err(InternalError::from_std)?
}
