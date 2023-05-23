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

//! Setup tracing on different platforms.

use std::fs::OpenOptions;

use serde::{Deserialize, Serialize};
use tracing::{error, Level};
use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    layer::SubscriberExt,
    util::{SubscriberInitExt, TryInitError},
};

use crate::utils::RelativePathBuf;

mod serde_level_filter {
    use serde::{
        de::{Deserialize, Deserializer, Error},
        ser::{Serialize, Serializer},
    };
    use tracing_subscriber::filter::LevelFilter;

    #[allow(clippy::trivially_copy_pass_by_ref)] // required by serde
    pub(super) fn serialize<S>(level: &LevelFilter, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        level.to_string().serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<LevelFilter, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).and_then(|level| {
            level
                .parse::<LevelFilter>()
                .map_err(|error| D::Error::custom(error.to_string()))
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub file: Option<RelativePathBuf>,
    #[serde(with = "serde_level_filter")]
    pub level: LevelFilter,
    pub install_panic_hook: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            file: None,
            level: LevelFilter::INFO,
            install_panic_hook: true,
        }
    }
}

/// Initializes the logging.
///
/// This should be called before [`crate::start()`] is called.
///
/// Even though this returns an error if logging was already initialized you
/// should only call this function when you expect it to succeed.
pub fn initialize(log_config: &Config) -> Result<(), TryInitError> {
    init_tracing_once(log_config)?;
    if log_config.install_panic_hook {
        init_panic_logging();
    }
    Ok(())
}

fn init_tracing_once(log_config: &Config) -> Result<(), TryInitError> {
    let subscriber = tracing_subscriber::registry();

    let stdout_log = tracing_subscriber::fmt::layer().with_ansi(false);

    let sqlx_query_no_info = Targets::new()
        .with_default(log_config.level)
        .with_target("sqlx::query", Level::WARN);

    let file_log = log_config
        .file
        .as_ref()
        .map(|log_file| {
            OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(log_file.relative())
                .map(|writer| {
                    tracing_subscriber::fmt::layer()
                        .with_writer(writer)
                        .with_ansi(false)
                        .json()
                })
        })
        .transpose()
        .map_err(|error| {
            eprintln!("Setup file logging failed: {error}");
        })
        .ok();

    subscriber
        .with(stdout_log)
        .with(sqlx_query_no_info)
        .with(file_log)
        .with(log_config.level)
        .try_init()
}

fn init_panic_logging() {
    std::panic::set_hook(Box::new(|panic| {
        if let Some(location) = panic.location() {
            error!(
                message = %panic,
                panic.file = location.file(),
                panic.line = location.line(),
                panic.column = location.column(),
            );
        } else {
            error!(message = %panic);
        }
    }));
}
