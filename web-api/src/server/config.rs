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

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use derive_more::{AsRef, Deref};
use serde::{Deserialize, Serialize};

use crate::{logging, storage};

/// Configuration for roughly network/connection layer specific configurations.
#[derive(Debug, Deserialize, Serialize)]
pub struct NetConfig {
    /// Address to which the server should bind.
    #[serde(default = "default_bind_address")]
    pub(crate) bind_to: SocketAddr,

    /// Max body size limit which should be applied to all endpoints
    #[serde(default = "default_max_body_size")]
    pub(crate) max_body_size: usize,
}

fn default_bind_address() -> SocketAddr {
    SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 4252).into()
}

fn default_max_body_size() -> usize {
    524_288
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            bind_to: default_bind_address(),
            max_body_size: default_max_body_size(),
        }
    }
}

/// Configuration combining all other configurations.
#[derive(AsRef, Debug, Deref, Deserialize, Serialize)]
pub struct Config<E> {
    #[as_ref]
    #[serde(default)]
    pub(crate) logging: logging::Config,

    #[as_ref]
    #[serde(default)]
    pub(crate) net: NetConfig,

    #[as_ref]
    #[serde(default)]
    pub(crate) storage: storage::Config,

    #[deref]
    #[serde(flatten)]
    pub(crate) extension: E,
}
