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

//! Setup tracing on different platform.

use std::{fs::OpenOptions, io, path::Path, sync::Once};

use tracing::metadata::LevelFilter;
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt};

static INIT_TRACING: Once = Once::new();

pub(crate) fn init_tracing(log_file: Option<&Path>) {
    INIT_TRACING.call_once(|| {
        init_tracing_once(log_file);
        init_panic_logging();
    });
}

fn init_tracing_once(log_file: Option<&Path>) {
    let stdout_log = tracing_subscriber::fmt::layer();

    let android_logging;
    cfg_if::cfg_if! {
        if #[cfg(target_os = "android")] {
            android_logging = tracing_android::layer("xayn_discovery_engine").ok()
        } else {
            // workaround to not have to specify a alternative type per hand
            android_logging = false.then(|| tracing_subscriber::fmt::layer())
        }
    };

    let file_log = log_file.map(|log_file| -> io::Result<_> {
        let writer = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(log_file)?;

        Ok(tracing_subscriber::fmt::layer()
            .with_writer(writer)
            .json()
            .with_span_events(FmtSpan::CLOSE))
    });

    let (file_log, file_error) = match file_log {
        None => (None, None),
        Some(Ok(log)) => (Some(log), None),
        Some(Err(err)) => (None, Some(err)),
    };

    tracing_subscriber::registry()
        .with(stdout_log)
        .with(android_logging)
        .with(file_log)
        //FIXME configurable add env logging?
        .with(LevelFilter::INFO)
        .init();

    if let Some(file_error) = file_error {
        tracing::error!(file_error=%file_error, "Initializing file logging faile");
    }
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
