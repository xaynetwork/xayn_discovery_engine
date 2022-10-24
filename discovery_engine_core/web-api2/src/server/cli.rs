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

/// Cli arguments for the web-api server.
#[derive(Parser, Debug, Serialize)]
#[command(author, version, about)]
pub(super) struct Args {
    /// Host and port to which the server should bind.
    ///
    /// This setting is prioritized over settings through
    /// the config and environment.
    #[arg(short, long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) bind_to: Option<SocketAddr>,

    /// File to log to additionally to logging to stdout.
    #[arg(short, long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) log_file: Option<PathBuf>,

    /// Use given configuration file.
    #[arg(short, long)]
    #[serde(skip)]
    pub(super) config: Option<PathBuf>,

    /// Print the config and exist instead of running the server
    #[arg(short, long)]
    #[serde(skip)]
    pub(super) print_config: bool,
}
