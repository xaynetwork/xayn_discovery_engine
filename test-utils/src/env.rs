// Copyright 2023 Xayn AG
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

use std::sync::Once;

use once_cell::sync::Lazy;
use regex::Regex;

/// Remove all variables from this process environment (with some exceptions).
///
/// Exceptions are following variables if well formed:
///
/// - `PATH`
/// - `LANG`
/// - `PWD`
/// - `CI`
/// - `RUST_BACKTRACE`
/// - `RUST_LIB_BACKTRACE`
///
/// Additional exceptions to avoid potential complications
/// with programs called by just, especially wrt to docker-compose
/// or podman-compose:
///
/// - `DBUS*`
/// - `SYSTEMD*`
/// - `USER*`
/// - `DOCKER*`
/// - `PODMAN*`
/// - `XDG*`
/// - `GITHUB_*`
/// - `XAYN_TEST_*`
pub fn clear_env() {
    static ONCE: Once = Once::new();
    // We need to make sure we only do it once as this doesn't execute concurrently
    // because `remove_var` is not reliably thread safe.
    ONCE.call_once(|| {
        for (key, _) in std::env::vars_os() {
            let keep = key
                .to_str()
                .map(|key| ENV_PRUNE_EXCEPTIONS.is_match(key))
                .unwrap_or_default();

            if !keep {
                std::env::remove_var(key);
            }
        }
    });
}

static ENV_PRUNE_EXCEPTIONS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        "(?ix)
        (?:^PATH$)
        |(?:^LANG$)
        |(?:^PWD$)
        |(?:^USER$)
        |(?:^CI$)
        |(?:^HOME$)
        |(?:^RUST_BACKTRACE$)
        |(?:^RUST_LIB_BACKTRACE$)
        |(?:^DBUS)
        |(?:^SYSTEMD)
        |(?:^DOCKER)
        |(?:^PODMAN)
        |(?:^XDG)
        |(?:^GITHUB_)
        |(?:^XAYN_TEST_)
        ",
    )
    .unwrap()
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_filter_regex() {
        let vars = [
            "ANDROID_SDK_HOME",
            "BINARYEN_ROOT",
            "CHROME_DESKTOP",
            "CHROME_EXECUTABL",
            "CLUTTER_BACKEND",
            "COLORTERM",
            "DBUS_SESSION_BUS_ADDRESS",
            "DEBUGINFOD_URLS",
            "DEFAULT",
            "DISPLAY",
            "CI",
            "ECORE_EVAS_ENGINE",
            "EDITOR",
            "ELM_ENGINE",
            "FLUTTER_HOME",
            "GDK_BACKEND",
            "GIT_ASKPASS",
            "GNOME_TERMINAL_SCREEN",
            "GNOME_TERMINAL_SERVICE",
            "GPG_TTY",
            "HOME",
            "I3SOCK",
            "LANG",
            "LOGNAME",
            "MAIL",
            "MOTD_SHOWN",
            "NO_AT_BRIDGE",
            "OLDPWD",
            "ORIGINAL_XDG_CURRENT_DESKTOP",
            "PATH",
            "PULSEMIXER_BAR_STYL",
            "PWD",
            "QT_QPA_PLATFORM",
            "QT_WAYLAND_DISABLE_WINDOWDECORATIONS",
            "SHELL",
            "SHLVL",
            "SSH_AUTH_SOCK",
            "STUDIO_JDK",
            "SWAYSOCK",
            "SYSTEMD_EXEC_PID",
            "TERM",
            "TERM_PROGRAM",
            "TERM_PROGRAM_VERSION",
            "USER",
            "VSCODE_GIT_ASKPASS_EXTRA_ARGS",
            "VSCODE_GIT_ASKPASS_MAIN",
            "VSCODE_GIT_ASKPASS_NODE",
            "VSCODE_GIT_IPC_HANDLE",
            "VTE_VERSION",
            "WAYLAND_DISPLAY",
            "XCURSOR_SIZE",
            "XDG_BIN_HOME",
            "XDG_CACHE_HOME",
            "XDG_CONFIG_DIRS",
            "XDG_CURRENT_DESKTOP",
            "XDG_DATA_DIRS",
            "XDG_DATA_HOME",
            "XDG_RUNTIME_DIR",
            "XDG_SEAT",
            "XDG_SESSION_CLASS",
            "XDG_SESSION_ID",
            "XDG_SESSION_TYPE",
            "XDG_VTNR",
            "DOCKER_DODO",
            "PODMAN_DODO",
            "_JAVA_AWT_WM_NONREPARENTING",
        ];

        let filtered = vars
            .into_iter()
            .filter(|key| ENV_PRUNE_EXCEPTIONS.is_match(key))
            .collect::<Vec<_>>();

        assert_eq!(
            filtered,
            [
                "DBUS_SESSION_BUS_ADDRESS",
                "CI",
                "HOME",
                "LANG",
                "PATH",
                "PWD",
                "SYSTEMD_EXEC_PID",
                "USER",
                "XDG_BIN_HOME",
                "XDG_CACHE_HOME",
                "XDG_CONFIG_DIRS",
                "XDG_CURRENT_DESKTOP",
                "XDG_DATA_DIRS",
                "XDG_DATA_HOME",
                "XDG_RUNTIME_DIR",
                "XDG_SEAT",
                "XDG_SESSION_CLASS",
                "XDG_SESSION_ID",
                "XDG_SESSION_TYPE",
                "XDG_VTNR",
                "DOCKER_DODO",
                "PODMAN_DODO",
            ]
        );
    }
}
