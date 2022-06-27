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

use std::{env::VarError, error::Error, fs::OpenOptions, io, path::Path, sync::Once};

use tracing::metadata::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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

    let subscriber = tracing_subscriber::registry();

    cfg_if::cfg_if! {
        if #[cfg(target_os = "android")] {
            let layer = tracing_android::layer("xayn_discovery_engine").ok();
            let subscriber = subscriber.with(layer);
        }
    };

    let file_log = log_file
        .map(|log_file| -> io::Result<_> {
            let writer = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(log_file)?;

            Ok(tracing_subscriber::fmt::layer().with_writer(writer).json())
        })
        .transpose()
        .map_err(|err| {
            delayed_errors.push(Box::new(err) as _);
        })
        .ok();

    let filter = build_env_filter(&mut delayed_errors);

    subscriber
        .with(stdout_log)
        .with(file_log)
        .with(filter)
        .init();

    for error in delayed_errors {
        tracing::error!(error=%error, "logging setup failed");
    }
}

/// Crates an [`EnvFilter`] based on the env of `RUST_LOG`.
///
/// If `RUST_LOG` is not set/empty `DISCOVERY_ENGINE_LOG` will
/// be used instead in the form of `info,xayn_=${DISCOVERY_ENGINE_LOG}`,
/// if that fails `info` is used.
fn build_env_filter(errors: &mut Vec<Box<dyn Error>>) -> EnvFilter {
    if let Some(directives) = env_var("RUST_LOG", errors) {
        match directives.parse() {
            Ok(val) => return val,
            Err(err) => {
                errors.push(Box::new(err));
            }
        }
    }

    if let Some(de_level) = env_var("DISCOVERY_ENGINE_LOG", errors) {
        match format!("info,xayn_={}", de_level).parse() {
            Ok(val) => return val,
            Err(err) => {
                errors.push(Box::new(err));
            }
        }
    }

    EnvFilter::default().add_directive(LevelFilter::INFO.into())
}

fn env_var(var: &str, errors: &mut Vec<Box<dyn Error>>) -> Option<String> {
    match std::env::var(var) {
        Ok(var) if !var.trim().is_empty() => return Some(var),
        Err(err @ VarError::NotUnicode(_)) => {
            errors.push(Box::new(err));
        }
        _ => (),
    }
    None
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
