// Copyright 2023 Xayn AG
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
use xayn_web_api_db_ctrl::tenant::Tenant;
use xayn_web_api_shared::elastic;

use crate::SetupError;

#[derive(Deref)]
pub(crate) struct Client(elastic::Client);

impl Client {
    pub(crate) fn builder(config: elastic::Config) -> Result<ClientBuilder, SetupError> {
        elastic::Client::new(config).map(ClientBuilder)
    }
}

#[derive(Clone)]
pub(crate) struct ClientBuilder(elastic::Client);

impl ClientBuilder {
    pub(crate) fn build_for(&self, tenant: &Tenant) -> Client {
        Client(self.0.with_index(&tenant.es_index_name))
    }
}
