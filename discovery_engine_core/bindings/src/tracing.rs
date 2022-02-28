//! Setup tracing on different platform.

use std::sync::Once;

use tracing_subscriber::{filter::LevelFilter, util::SubscriberInitExt};

static INIT_TRACING: Once = Once::new();

pub(crate) fn init_tracing() {
    INIT_TRACING.call_once(|| {
        init_tracing_once();
    });
}

fn init_tracing_once() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .finish();

    cfg_if::cfg_if! {
        if #[cfg(target_os = "android")] {
            use tracing_subscriber::layer::SubscriberExt;
            let layer = tracing_android::layer("xayn_discovery_engine").ok();
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
