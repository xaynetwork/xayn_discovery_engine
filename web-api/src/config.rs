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

mod cli;
mod load_config;

use std::{ffi::OsString, process::exit};

use clap::{CommandFactory, Parser};
use serde::{de::DeserializeOwned, Serialize};

use self::{cli::Args, load_config::load_config};

/// Loads the config with custom CLI args.
///
/// See [`Config.load()`].
#[allow(dead_code)]
pub fn load_with_args<C>(
    application_names: &[&str],
    args: impl IntoIterator<Item = impl Into<OsString> + Clone>,
) -> C
where
    C: Serialize + DeserializeOwned,
{
    load_with_parsed_args(application_names, Args::parse_from(args))
}

/// Loads the config.
///
/// # Panic/Program Exit
///
/// In case of `--help`, `--print-config` and failure
/// this functions will not return normally but terminate
/// the program normally instead.
pub fn load<C>(application_names: &[&str]) -> C
where
    C: Serialize + DeserializeOwned,
{
    load_with_parsed_args(application_names, Args::parse())
}

fn load_with_parsed_args<C>(application_names: &[&str], mut cli_args: Args) -> C
where
    C: Serialize + DeserializeOwned,
{
    let config = cli_args.config.take();
    let config = match load_config(
        application_names,
        config.as_deref(),
        cli_args.to_config_overrides(),
    ) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Error: {err}");
            cli::Args::command().print_help().ok();
            exit(1);
        }
    };

    if cli_args.print_config {
        println!("{}", serde_json::to_string_pretty(&config).unwrap());
        exit(0);
    }

    config
}
