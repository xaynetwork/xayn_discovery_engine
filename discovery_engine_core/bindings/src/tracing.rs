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

use std::{fs::OpenOptions, path::Path, sync::Once};

use tracing::Level;
use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

static INIT_TRACING: Once = Once::new();

pub(crate) fn init_tracing(log_file: Option<&Path>) {
    INIT_TRACING.call_once(|| {
        init_tracing_once(log_file);
        init_panic_logging();
    });
}

fn init_tracing_once(log_file: Option<&Path>) {
    let stdout_log = tracing_subscriber::fmt::layer();

    let subscriber = tracing_subscriber::registry();

    //FIXME fix log capturing for dart integration tests inste
    let sqlx_query_no_info = Targets::new().with_target("sqlx::query", Level::WARN);

    cfg_if::cfg_if! {
        if #[cfg(target_os = "android")] {
            let layer = tracing_android::layer("xayn_discovery_engine").ok();
            let subscriber = subscriber.with(layer);
        }
    };

    let file_log = log_file
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
            tracing::error!(%error, "logging setup failed");
        })
        .ok();

    let level = if log_file.is_some() {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    subscriber
        .with(stdout_log)
        .with(sqlx_query_no_info)
        .with(file_log)
        .with(level)
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
