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

use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt};

static INIT_TRACING: Once = Once::new();

pub(crate) fn init_tracing(log_file: Option<&Path>) {
    INIT_TRACING.call_once(|| {
        init_tracing_once(log_file);
        init_panic_logging();
    });
}

fn init_tracing_once(log_file: Option<&Path>) {
    let max_level = LevelFilter::INFO;

    let error = if let Some(log_file) = log_file {
        match init_with_file_logging(max_level, log_file) {
            Ok(()) => return,
            Err(err) => Some(err),
        }
    } else {
        None
    };

    init_normal_logging(max_level);

    if let Some(error) = error {
        tracing::error!("Initializing file logging faile: {}", error);
    }
}

fn init_with_file_logging(max_level: LevelFilter, log_file: &Path) -> io::Result<()> {
    let writer = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(log_file)?;

    let layer = tracing_subscriber::fmt::layer()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .with_ansi(false)
        .with_writer(writer);

    tracing_subscriber::fmt()
        .with_max_level(max_level)
        .finish()
        .with(layer)
        .init();

    Ok(())
}

fn init_normal_logging(max_level: LevelFilter) {
    let subscriber = tracing_subscriber::fmt().with_max_level(max_level).finish();

    cfg_if::cfg_if! {
        if #[cfg(target_os = "android")] {
            use tracing_subscriber::layer::SubscriberExt;
            let layer = tracing_android::layer("xayn_discovery_engine").ok();
            subscriber.with(layer).init();
        } else {
            subscriber.init();
        }
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
