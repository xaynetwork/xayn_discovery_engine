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

use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt};

static INIT_TRACING: Once = Once::new();

pub(crate) fn init_tracing(log_file: Option<&Path>) {
    INIT_TRACING.call_once(|| {
        init_tracing_once(log_file);
        init_panic_logging();
    });
}

fn init_tracing_once(log_file: Option<&Path>) {
    let subscriber = tracing_subscriber::registry();

    let stdout_log = tracing_subscriber::fmt::layer();

    // let sqlx_query_no_info = Targets::new().with_target("sqlx::query", Level::WARN);

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
            eprintln!("Setup file logging failed: {}", error);
        })
        .ok();

    subscriber
        .with(stdout_log)
        //FIXME this doesn't seem to work correctly
        // .with(sqlx_query_no_info)
        .with(file_log)
        //FIXME use env to determine logging level
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
