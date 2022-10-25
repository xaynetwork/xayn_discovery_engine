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

use derive_more::{AsRef, Deref};
use sqlx::{Pool, Postgres};

use super::{Config, SetupError};

#[derive(Deref, AsRef)]
pub struct AppState<CE, AE> {
    #[allow(dead_code)]
    #[as_ref]
    pub(crate) config: Config<CE>,
    #[allow(dead_code)]
    #[as_ref]
    pub(crate) db: Pool<Postgres>,
    #[deref]
    pub(crate) extension: AE,
}

impl<CE, AE> AppState<CE, AE> {
    pub(super) async fn create(
        config: Config<CE>,
        create_extension: impl FnOnce(&Config<CE>) -> Result<AE, SetupError>,
    ) -> Result<Self, SetupError> {
        let db = config.db.create_connection_pool().await?;
        let extension = create_extension(&config)?;
        // let embedder = Embedder::load(config.as_ref())?;
        // let coi = config.extension.personalization.coi.build();
        Ok(Self {
            config,
            db,
            extension,
        })
    }
}
