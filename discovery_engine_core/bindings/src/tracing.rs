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

use std::sync::Once;

use tracing_subscriber::{filter::LevelFilter, util::SubscriberInitExt};

static INIT_TRACING: Once = Once::new();

pub(crate) fn init_tracing(path: &str) {
    INIT_TRACING.call_once(|| {
        init_tracing_once(path);
    });
}

#[allow(unused_variables)]
fn init_tracing_once(path: &str) {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .finish();

    cfg_if::cfg_if! {
        if #[cfg(target_os = "android")] {
            use tracing_subscriber::layer::SubscriberExt;
            let layer = tracing_android::layer("xayn_discovery_engine").ok();
            subscriber.with(layer).init();
        } else if #[cfg(target_os = "ios")]  {
            use tracing_subscriber::layer::SubscriberExt;
            let appender = tracing_appender::rolling::never(path, "tracing_engine.log");
            let layer = tracing_subscriber::fmt::layer()
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
                .with_ansi(false)
                .with_writer(appender);
            subscriber.with(layer).init();
        } else {
            subscriber.init();
        }
    }

    log_panic();
}

fn log_panic() {
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
