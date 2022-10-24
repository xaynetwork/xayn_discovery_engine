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

use std::{fs::OpenOptions, path::PathBuf, sync::Once};

use serde::Deserialize;
use tracing::Level;
use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

#[derive(Deserialize, Default, Debug)]
pub struct Config {
    pub(crate) file: Option<PathBuf>,
}

static INIT_TRACING: Once = Once::new();

pub(crate) fn init_tracing(log_config: &Config) {
    INIT_TRACING.call_once(|| {
        init_tracing_once(log_config);
        init_panic_logging();
    });
}

fn init_tracing_once(log_config: &Config) {
    let subscriber = tracing_subscriber::registry();

    let stdout_log = tracing_subscriber::fmt::layer();

    let sqlx_query_no_info = Targets::new()
        // trace => do not affect filtering of any other targets
        .with_default(LevelFilter::TRACE)
        .with_target("sqlx::query", Level::WARN);

    let file_log = log_config
        .file
        .as_deref()
        .map(|log_file| {
            OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(log_file)
                .map(|writer| tracing_subscriber::fmt::layer().with_writer(writer).json())
        })
        .transpose()
        .map_err(|error| {
            eprintln!("Setup file logging failed: {}", error);
        })
        .ok();

    subscriber
        .with(stdout_log)
        .with(sqlx_query_no_info)
        .with(file_log)
        //FIXME[ET-3444] use env to determine logging level
        .with(LevelFilter::DEBUG)
        .init();
}

fn init_panic_logging() {
    std::panic::set_hook(Box::new(|panic| {
        if let Some(location) = panic.location() {
            tracing::error!(
                message = %panic,
                panic.file = location.file(),
                panic.line = location.line(),
                panic.column = location.column(),
            );
        } else {
            tracing::error!(message = %panic);
        }
    }));
}
