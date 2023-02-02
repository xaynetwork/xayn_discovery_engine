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

use std::{net::SocketAddr, path::PathBuf};

use clap::Parser;
use serde::Serialize;
use serde_json::{json, Map, Value};

/// Cli arguments for the web-api server.
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub(super) struct Args {
    /// Host and port to which the server should bind.
    ///
    /// This setting is prioritized over settings through
    /// the config and environment.
    #[arg(short, long)]
    pub(super) bind_to: Option<SocketAddr>,

    /// File to log to additionally to logging to stdout.
    #[arg(short, long)]
    pub(super) log_file: Option<PathBuf>,

    /// Use given configuration file.
    ///
    /// Instead of a path "inline" toml configuration file can also be
    /// passed in by prefixing it with `inline:`.
    #[arg(short, long)]
    pub(super) config: Option<String>,

    /// Print the config and exist instead of running the server
    #[arg(long)]
    pub(super) print_config: bool,
}

impl Args {
    pub(super) fn to_config_overrides(&self) -> impl Serialize {
        let mut map = Map::new();
        if let Some(bind_to) = &self.bind_to {
            map.insert(String::from("net"), json!({ "bind_to": bind_to }));
        }
        if let Some(log_file) = &self.log_file {
            map.insert(String::from("logging"), json!({ "file": log_file }));
        }

        Value::Object(map)
    }
}
