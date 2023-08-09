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

use std::{
    collections::HashMap,
    env,
    ffi::{OsStr, OsString},
    sync::Mutex,
};

use trycmd::TestCases;

fn with_env_guard(
    env: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
    test: impl Fn(),
) {
    static GUARD: Mutex<()> = Mutex::new(());

    let guard = GUARD.lock();
    for (key, _) in env::vars_os() {
        if let Some(key_str) = key.to_str() {
            if key_str.as_bytes().starts_with(b"XAYN_") {
                env::remove_var(key);
            }
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
    with_env_guard(no_env(), || {
        TestCases::new().case("tests/cmd/*.auto.toml");
    });
}

#[test]
fn test_loading_config_with_env_overrides() {
    with_env_guard(
        [
            ("XAYN_WEB_API__STORAGE__POSTGRES__PORT", "3532"),
            ("XAYN_WEB_API__LOGGING__LEVEL", "trace"),
            ("XAYN_WEB_API__TENANTS__ENABLE_LEGACY_TENANT", "false"),
        ],
        || {
            TestCases::new().case("tests/cmd/env_overrides.toml");
        },
    );
}

#[test]
fn test_loading_config_with_mixed_overrides() {
    with_env_guard([("XAYN_WEB_API__NET__BIND_TO", "127.10.10.9:4343")], || {
        TestCases::new().case("tests/cmd/mixed_overrides.toml");
    });
}
