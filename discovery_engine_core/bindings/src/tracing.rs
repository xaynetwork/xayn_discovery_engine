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

use std::{error::Error, fs::OpenOptions, io, path::Path, sync::Once};

use tracing::metadata::LevelFilter;
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
    EnvFilter,
};

static INIT_TRACING: Once = Once::new();

pub(crate) fn init_tracing(log_file: Option<&Path>) {
    INIT_TRACING.call_once(|| {
        init_tracing_once(log_file);
        init_panic_logging();
    });
}

fn init_tracing_once(log_file: Option<&Path>) {
    let mut delayed_errors = Vec::new();

    let stdout_log = tracing_subscriber::fmt::layer();

    let android_logging;
    cfg_if::cfg_if! {
        if #[cfg(target_os = "android")] {
            //TODO error
            android_logging = tracing_android::layer("xayn_discovery_engine").ok()
        } else {
            // workaround to not have to specify a alternative type per hand
            android_logging = false.then(|| tracing_subscriber::fmt::layer())
        }
    };

    let file_log = log_file
        .map(|log_file| -> io::Result<_> {
            let writer = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(log_file)?;

            Ok(tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .json()
                .with_span_events(FmtSpan::CLOSE))
        })
        .transpose()
        .map_err(|err| {
            delayed_errors.push(Box::new(err) as _);
            ()
        })
        .ok();

    let filter = build_env_filter(&mut delayed_errors);

    tracing_subscriber::registry()
        .with(stdout_log)
        .with(android_logging)
        .with(file_log)
        .with(filter)
        .init();

    for error in delayed_errors {
        tracing::error!(error=%error, "logging setup failed");
    }
}

/// Crates an [`EnvFilter`] based on the env of `RUST_LOG`.
///
/// If `RUST_LOG` is empty/not set it will fall back to
/// `info` level, malformed directives are ignored but
/// and error is logged after logging is setup.
fn build_env_filter(errors: &mut Vec<Box<dyn Error>>) -> EnvFilter {
    let env_builder = EnvFilter::builder().with_default_directive(LevelFilter::INFO.into());

    env_builder.from_env().unwrap_or_else(|err| {
        errors.push(Box::new(err));
        env_builder.from_env_lossy()
    })
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
