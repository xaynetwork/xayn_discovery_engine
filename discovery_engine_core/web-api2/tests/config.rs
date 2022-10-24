use std::{
    collections::HashMap,
    env,
    ffi::{OsStr, OsString},
    os::unix::prelude::OsStrExt,
    sync::Mutex,
};
use trycmd::TestCases;

fn env_guard(
    env: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
    test: impl Fn(),
) {
    static GUARD: Mutex<()> = Mutex::new(());

    let guard = GUARD.lock();
    for (key, _) in env::vars_os() {
        if key.as_bytes().starts_with(b"XAYN_") {
            env::remove_var(key);
        }
    }

    for (key, value) in env.into_iter() {
        env::set_var(key.as_ref(), value.as_ref());
    }

    test();

    drop(guard)
}

fn no_env() -> HashMap<OsString, OsString> {
    HashMap::new()
}

#[test]
fn test_loading_configs() {
    env_guard(no_env(), || {
        TestCases::new().case("tests/cmd/*.auto.toml");
    });
}

#[test]
fn test_loading_config_with_env_overrides() {
    env_guard(
        [
            ("XAYN_WEB_API__DB__PORT", "3532"),
            ("XAYN_WEB_API__NET__MAX_BODY_SIZE", "4422"),
        ],
        || {
            TestCases::new().case("tests/cmd/env_overrides.toml");
        },
    );
}

#[test]
fn test_loading_config_with_mixed_overrides() {
    env_guard([("XAYN_WEB_API__NET__BIND_TO", "127.10.10.9:4343")], || {
        TestCases::new().case("tests/cmd/mixed_overrides.toml");
    });
}
