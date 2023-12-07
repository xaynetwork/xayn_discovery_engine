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

use std::{ffi::OsString, fmt::Display, path::Path, process::exit};

use clap::{CommandFactory, Parser};
use derive_more::AsRef;
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use xayn_ai_coi::CoiConfig;

use self::cli::Args;
use crate::{
    backoffice::IngestionConfig,
    embedding,
    extractor,
    frontoffice::{PersonalizationConfig, SemanticSearchConfig},
    logging,
    net,
    storage::{self},
    tenants,
    SetupError,
};

#[derive(AsRef, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct Config {
    pub(crate) logging: logging::Config,
    pub(crate) net: net::Config,
    pub(crate) storage: storage::Config,
    pub(crate) coi: CoiConfig,
    pub(crate) embedding: embedding::Config,
    pub(crate) text_extractor: extractor::Config,
    pub(crate) personalization: PersonalizationConfig,
    pub(crate) semantic_search: SemanticSearchConfig,
    pub(crate) ingestion: IngestionConfig,
    pub(crate) snippet_extractor: xayn_snippet_extractor::Config,
    pub(crate) tenants: tenants::Config,
}

impl Config {
    /// Loads the config.
    ///
    /// # Panic/Program Exit
    ///
    /// In case of `--help`, `--print-config` and failure
    /// this functions will not return normally but terminate
    /// the program normally instead.
    pub fn load(application_names: impl IntoIterator<Item = impl Display>) -> UnvalidatedConfig {
        load_with_parsed_args(application_names, Args::parse())
    }

    /// Loads the config with custom CLI args.
    ///
    /// See [`load()`].
    pub fn load_with_args(
        application_names: impl IntoIterator<Item = impl Display>,
        args: impl IntoIterator<Item = impl Into<OsString> + Clone>,
    ) -> UnvalidatedConfig {
        load_with_parsed_args(application_names, Args::parse_from(args))
    }
}

pub struct UnvalidatedConfig {
    config: Config,
    print_config: bool,
}

impl UnvalidatedConfig {
    pub fn logging_config(&self) -> &logging::Config {
        self.config.as_ref()
    }

    /// Finalizes the config doing an post deserialization validation steps.
    ///
    /// If the `--print-config` CLI arg was used a JSON serialization of the config
    /// will be printed to stdout. If additionally `exit_on_print` was set the program
    /// will exit with a success status code after printing.
    pub fn finalize(self, exit_on_print: bool) -> Result<Config, SetupError> {
        let Self {
            config,
            print_config,
        } = self;
        config.ingestion.validate()?;
        config.personalization.validate()?;
        config.semantic_search.validate()?;

        if print_config {
            println!("{}", serde_json::to_string_pretty(&config)?);
            if exit_on_print {
                exit(0)
            }
        }
        Ok(config)
    }
}

fn load_with_parsed_args(
    application_names: impl IntoIterator<Item = impl Display>,
    mut cli_args: Args,
) -> UnvalidatedConfig {
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

    UnvalidatedConfig {
        config,
        print_config: cli_args.print_config,
    }
}

/// Load the configuration into given type.
///
/// # Load order/priority
///
/// This will by ascending priority load:
///
/// 1. `./config.toml` or specified toml config file
/// 2. `./.env`
/// 3. `./.env.local`
/// 4. process environment
/// 5. options passed through `update_with`
///
/// Config values loaded from higher priority sources override such from lower
/// priority sources. E.g. values defined in `update_with` override values
/// from any other source.
///
/// `.env` is included to avoid confusion with env variables missing when calling
/// cargo directly instead of indirectly through `just`.
///
/// `.env.local` is a semi-standard way to add temporary local overrides that you
/// don't want to commit.
///
/// # Env and .env
///
/// Environment variables from `.env` and `.env.local` will be loaded into the process
/// environment if they don't already exist there (keeping priority as described above).
///
/// When creating the config type instance, only environment variables which start with
/// one of the names passed in `application_names` will be considered (case insensitive).
///
/// Variables with names earlier in the array take priority over variables with names
/// later in the array.
///
/// Env variable are converted into a config path by splitting it at `__` (and stripping
/// the application name). E.g. `XAYN_WEB_API__FOO__BAR=12` will be treated like
/// the json `{ "foo": { "bar": 12 } }` wrt. deserializing the config if `XAYN_WEB_API` is
/// in `application_names`.
fn load_config<C, U>(
    application_names: impl IntoIterator<Item = impl Display>,
    config: Option<&str>,
    update_with: U,
) -> Result<C, figment::Error>
where
    C: DeserializeOwned,
    U: Serialize,
{
    // the order must be from highest to lowest priority
    // or else it won't work correctly
    //FIXME figment Provider for .env, but it's annoying due to side effects
    load_dotenv(".env.local")?;
    load_dotenv(".env")?;

    let mut figment = Figment::new().join(Serialized::defaults(update_with));

    for name in application_names {
        figment = figment.join(Env::prefixed(&format!("{name}__")).split("__"));
    }

    let provider = config
        .map(|content_or_path| {
            if let Some(content) = content_or_path.strip_prefix("inline:") {
                Toml::string(content)
            } else {
                Toml::file(content_or_path)
            }
        })
        .or_else(|| {
            let default_file = Path::new("config.toml");
            default_file.exists().then(|| Toml::file(default_file))
        });

    if let Some(provider) = provider {
        figment = figment.join(provider);
    }

    figment.extract().map_err(Into::into)
}

fn load_dotenv(file_name: &str) -> Result<(), figment::Error> {
    match dotenvy::from_filename(file_name) {
        Err(error) if !error.not_found() => {
            Err(figment::Error::from(error.to_string()).with_path(file_name))
        }
        _ => Ok(()),
    }
}
