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

use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use serde::{Deserialize, Serialize};

mod serde_duration_as_seconds {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(super) fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        u64::deserialize(deserializer).map(Duration::from_secs)
    }
}

/// Configuration for roughly network/connection layer specific configurations.
#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Address to which the server should bind.
    pub(crate) bind_to: SocketAddr,

    /// Max body size limit which should be applied to all endpoints
    pub(crate) max_body_size: usize,

    /// Keep alive timeout in seconds
    #[serde(with = "serde_duration_as_seconds")]
    pub(crate) keep_alive: Duration,

    /// Client request timeout in seconds
    #[serde(with = "serde_duration_as_seconds")]
    pub(crate) client_request_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_to: SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 4252).into(),
            max_body_size: 524_288,
            keep_alive: Duration::from_secs(61),
            client_request_timeout: Duration::from_secs(0),
        }
    }
}
