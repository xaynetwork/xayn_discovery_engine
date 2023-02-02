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

use crate::app::{Application, SetupError};

#[derive(AsRef, Deref)]
pub(crate) struct AppState<A>
where
    A: Application,
{
    #[as_ref(forward)]
    pub(crate) config: A::Config,
    #[deref]
    pub(crate) extension: A::Extension,
    pub(crate) storage: A::Storage,
}

impl<A> AppState<A>
where
    A: Application,
{
    pub(super) async fn create(config: A::Config) -> Result<Self, SetupError> {
        let extension = A::create_extension(&config)?;
        let storage = A::setup_storage(config.as_ref()).await?;

        Ok(Self {
            config,
            extension,
            storage,
        })
    }
}
