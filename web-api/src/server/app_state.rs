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

use derive_more::Deref;

use crate::{
    server::{Config, SetupError},
    storage::Storage,
};

#[derive(Deref)]
pub(crate) struct AppState<C, E, S> {
    pub(crate) config: C,
    #[deref]
    pub(crate) extension: E,
    pub(crate) storage: S,
}

impl<C, E> AppState<C, E, Storage>
where
    C: Config,
{
    pub(super) async fn create(
        config: C,
        create_extension: impl FnOnce(&C) -> Result<E, SetupError>,
    ) -> Result<Self, SetupError> {
        let extension = create_extension(&config)?;
        let storage = config.storage().setup().await?;

        Ok(Self {
            config,
            extension,
            storage,
        })
    }
}
