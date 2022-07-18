// Copyright 2022 Xayn AG
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Provides ways to handle the async runtime used by async-bindgen.
use std::future::Future;
use tokio::runtime::{Handle, Runtime};

use once_cell::sync::OnceCell;

static RUNTIME: OnceCell<Runtime> = OnceCell::new();

/// Runs a closure inside of a runtime.
///
/// If we already are inside of a runtime, that runtime
/// is used.
///
/// If we are not in a runtime a new runtime is created.
///
/// For now there is no way to interact with that runtime besides
/// this method, similar there is for now no way to set an externally
/// created runtime.
///
/// # Panics
///
/// Panics if a runtime needs to be created and that fails.
///
/// Normally creating a runtime doesn't fail.
///
/// Though `tokio` doesn't document when runtime creation
/// can fail.
pub fn with_runtime<R>(run: impl FnOnce() -> R) -> R {
    if Handle::try_current().is_ok() {
        run()
    } else {
        let rt = RUNTIME.get_or_init(|| Runtime::new().expect("creating tokio runtime failed"));
        let handle = rt.handle();
        let guard = handle.enter();
        let r = run();
        drop(guard);
        r
    }
}

/// Spawns a future on a runtime.
///
/// # Panics
///
/// Panics if a runtime needs to be created and that fails.
///
/// Normally creating a runtime doesn't fail.
pub fn spawn(future: impl Future<Output = ()> + Send + 'static) {
    with_runtime(|| tokio::spawn(future));
}
