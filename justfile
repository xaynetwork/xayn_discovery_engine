# We import environment variables from .env
set dotenv-load := true
set shell := ["bash", "-euxc", "-o", "pipefail"]

# If CI and DENY_WARNINGS are set add rust flags
export RUSTFLAGS := if env_var_or_default("CI", "false") == "true" {
    if env_var_or_default("DENY_WARNINGS", "true") == "true" {
        trim(env_var_or_default("RUSTFLAGS", "") + " -D warnings")
    } else {
        env_var_or_default("RUSTFLAGS", "")
    }
} else {
    env_var_or_default("RUSTFLAGS", "")
}
export RUSTDOCFLAGS := if env_var_or_default("CI", "false") == "true" {
    if env_var_or_default("DENY_WARNINGS", "true") == "true" {
        trim(env_var_or_default("RUSTDOCFLAGS", "") + " -D warnings")
    } else {
        env_var_or_default("RUSTDOCFLAGS", "")
    }
} else {
    env_var_or_default("RUSTDOCFLAGS", "")
}

# Runs just --list
default:
    @{{just_executable()}} --list

# Gets/updates dart deps
_dart-deps $WORKSPACE:
    cd "$WORKSPACE" && dart pub get

dart-deps:
    @{{just_executable()}} _dart-deps "$DART_WORKSPACE"
    @{{just_executable()}} _dart-deps "$DART_WORKSPACE/example"
    @{{just_executable()}} _dart-deps "$BINDGEN_DART_WORKSPACE"

# Gets/updates flutter project deps
flutter-deps:
    #!/usr/bin/env bash
    set -eux -o pipefail
    cd "$FLUTTER_WORKSPACE";
    flutter pub get
    cd example
    flutter pub get

# Fetches rust dependencies
rust-deps:
    cd "$RUST_WORKSPACE"; \
    cargo fetch {{ if env_var_or_default("CI", "false") == "true" { "--locked" } else { "" } }}

# Get/Update/Fetch/Install all dependencies
deps: flutter-deps dart-deps rust-deps

_dart-fmt $WORKSPACE:
    cd "$WORKSPACE"; \
    dart format {{ if env_var_or_default("CI", "false") == "true" { "--output=none --set-exit-if-changed" } else { "" } }} .

# Formats dart (checks only on CI)
dart-fmt:
    @{{just_executable()}} _dart-fmt "$DART_WORKSPACE"
    @{{just_executable()}} _dart-fmt "$FLUTTER_WORKSPACE"
    @{{just_executable()}} _dart-fmt "$BINDGEN_DART_WORKSPACE"

# Formats rust (checks only on CI)
rust-fmt:
    #!/usr/bin/env bash
    set -eux -o pipefail
    cd "$RUST_WORKSPACE";
    cargo +"$RUST_NIGHTLY" fmt --all -- {{ if env_var_or_default("CI", "false") == "true" { "--check" } else { "" } }};
    cargo sort --grouped --workspace {{ if env_var_or_default("CI", "false") == "true" { "--check" } else { "" } }}

# Formats all code (checks only on CI)
fmt: rust-fmt dart-fmt

_dart-analyze $WORKSPACE:
    cd "$WORKSPACE"; \
    dart analyze --fatal-infos

# Checks dart code, fails on info on CI
dart-check: dart-build
    @{{just_executable()}} _dart-analyze "$DART_WORKSPACE"
    @{{just_executable()}} _dart-analyze "$BINDGEN_DART_WORKSPACE"

flutter-check: dart-build flutter-deps
    @{{just_executable()}} _dart-analyze "$FLUTTER_WORKSPACE"

flutter-test: dart-build flutter-deps
    cd "$FLUTTER_WORKSPACE"; \
    flutter test

# async-bindgen generates extern C functions which
# cbindgen needs to process, but cbindgen can't see them
# without using a nightly rust feature. But we are fixed
# to stable (it also does more then we need, making it rather
# slow). As a work around we currently write the generated
# code into a file, but that happens after cbindgen/rust
# parsed all files, so we need to run cargo check once
# before we need it. When we have time this will be
# replaced by a better workaround.
_codegen-order-workaround:
    cd "$RUST_WORKSPACE"; \
    cargo check --quiet 2>/dev/null || :

# Checks rust code, fails on warnings on CI
rust-check: _codegen-order-workaround
    cd "$RUST_WORKSPACE"; \
    cargo clippy --all-targets --locked; \
    cargo clippy --all-targets --features storage --locked; \
    cargo check -p xayn-discovery-engine-bindings

# Checks rust and dart code, fails if there are any issues on CI
check: rust-check dart-check flutter-check

# Checks if dart documentation can be build without issues
dart-check-doc: dart-build
    cd "$DART_WORKSPACE"; \
    dart doc --verbose --dry-run --validate-links

# Checks if rust documentation can be build without issues
rust-check-doc: _codegen-order-workaround
    cd "$RUST_WORKSPACE"; \
    cargo doc --all-features --no-deps --document-private-items --locked

# Builds dart documentation
dart-doc *args: dart-build
    cd "$DART_WORKSPACE"; \
    dart doc {{args}}

# Builds rust documentation
rust-doc *args:
    cd "$RUST_WORKSPACE"; \
    cargo doc --all-features --document-private-items --locked {{args}}

# Builds rust and dart documentation
doc: dart-doc rust-doc

# Checks if documentation can be build without issues
check-doc: dart-check-doc rust-check-doc

_run-cbindgen: _codegen-order-workaround
    cd "$RUST_WORKSPACE"; \
    cargo check

_run-ffigen:
    cd "$DART_WORKSPACE"; \
    dart run ffigen --config ffigen.yaml

_run-async-bindgen:
    cd "$RUST_WORKSPACE" && \
    cargo run -p async-bindgen-gen-dart -- \
        --ffi-class XaynDiscoveryEngineBindingsFfi \
        --genesis ../"$DART_WORKSPACE"/lib/src/ffi/genesis.ffigen.dart

_run-build-runner:
    cd "$DART_WORKSPACE"; \
    dart run build_runner build --delete-conflicting-outputs

# Builds dart (runs all codegen steps)
dart-build: _run-cbindgen _run-ffigen _run-async-bindgen _run-build-runner

# Builds rust
rust-build: _codegen-order-workaround
    cd "$RUST_WORKSPACE"; \
    cargo build --locked

# Builds dart and rust
build: rust-build dart-build

# Tests dart (builds all necessary parts)
dart-test: rust-build dart-build download-assets
    cd "$DART_WORKSPACE"; \
    dart test

# Tests rust
rust-test: _codegen-order-workaround download-assets
    #!/usr/bin/env bash
    set -eux -o pipefail
    cd "$RUST_WORKSPACE";
    cargo test --lib --bins --tests --quiet --locked
    cargo test --doc --quiet --locked
    cargo test --lib --features storage --quiet --locked

# Tests dart and rust
test: rust-test dart-test flutter-test

# Cleans up all generated files
clean-gen-files:
    find . \( -name '*.g.dart' \
        -or -name '*.freezed.dart' \
        -or -name '*.ffigen.dart' \
        -or -name '*.ext.dart' \
    \) -exec rm '{}' \;
    -rm "$RUST_WORKSPACE"/bindings/include/*
    -rm "$RUST_WORKSPACE"/bindings/src/async_bindings/*

# Cleans up rusts build cache
rust-clean:
    cd "$RUST_WORKSPACE"; \
    cargo clean

# Cleans up darts build cache
_dart-clean $WORKSPACE:
    cd "$WORKSPACE"; \
    find . -type d -name .dart_tool -prune -exec rm -r '{}' \;

dart-clean:
    @{{just_executable()}} _dart-clean "$DART_WORKSPACE"
    @{{just_executable()}} _dart-clean "$BINDGEN_DART_WORKSPACE"
    @{{just_executable()}} _dart-clean "$FLUTTER_WORKSPACE"

# Removes all local cargo installs
clean-tools:
    -rm -r "$CARGO_INSTALL_ROOT"

# Removes all asset data
remove-assets:
    find $FLUTTER_EXAMPLE_WORKSPACE/assets/*_v* -type f ! -name .gitkeep ! -name '*-mocked.onnx' -exec rm '{}' \;

# Removes all local cached dependencies and generated files
clean: clean-gen-files rust-clean dart-clean

# Runs clean and removes local installed tools
clean-fully: clean clean-tools remove-assets

# Workaround to set env variable CI for all job dependencies
_pre-push: deps clean-gen-files fmt check test

# Runs formatting, checks and test steps after deleting generated files.
pre-push $CI="true":
    @{{just_executable()}} _pre-push

_compile-android target:
    # See also: https://developer.android.com/studio/projects/gradle-external-native-builds#jniLibs
    cd "$RUST_WORKSPACE"; \
        cargo ndk --bindgen -t $(echo "{{target}}" | sed 's/[^ ]* */&/g') -p $ANDROID_PLATFORM_VERSION \
        -o "{{justfile_directory()}}/$FLUTTER_WORKSPACE/android/src/main/jniLibs" build \
        --release -p xayn-discovery-engine-bindings --locked

compile-android-local: _codegen-order-workaround
    #!/usr/bin/env bash
    set -eux -o pipefail
    cd "$RUST_WORKSPACE";
    for TARGET in $ANDROID_TARGETS; do
        {{just_executable()}} _compile-android $TARGET
    done

compile-android-ci target prod_flag="\"\"": _codegen-order-workaround
    #!/usr/bin/env bash
    set -eux -o pipefail
    if [[ {{prod_flag}} == "--prod" ]]; then
        RUSTFLAGS=$PRODUCTION_RUSTFLAGS {{just_executable()}} _compile-android {{target}}
    else
        {{just_executable()}} _compile-android {{target}}
    fi

# Compiles the bindings for the given iOS target
_compile-ios target:
    cd "$RUST_WORKSPACE"; \
    cargo build --target {{target}} -p xayn-discovery-engine-bindings --release --locked

# Compiles the bindings for iphoneos (aarch64) and iphonesimulator (x86_64)
# and copies the binaries to the flutter project
compile-ios-local: _codegen-order-workaround
    #!/usr/bin/env bash
    set -eux -o pipefail
    for TARGET in $IOS_TARGETS; do
        {{just_executable()}} _compile-ios $TARGET
        cp "$RUST_WORKSPACE/target/$TARGET/release/${IOS_LIB_BASE}.a" "$FLUTTER_WORKSPACE/ios/${IOS_LIB_BASE}_${TARGET}.a"
    done

compile-ios-ci target prod_flag="\"\"": _codegen-order-workaround
    #!/usr/bin/env bash
    set -eux -o pipefail
    if [[ {{prod_flag}} == "--prod" ]]; then
        RUSTFLAGS=$PRODUCTION_RUSTFLAGS {{just_executable()}} _compile-ios {{target}}
        strip -S -x -r "$RUST_WORKSPACE/target/{{target}}/release/${IOS_LIB_BASE}.a"
    else
        {{just_executable()}} _compile-ios {{target}}
    fi

flutter-run: dart-build
    cd "$FLUTTER_EXAMPLE_WORKSPACE" && \
        flutter run

flutter-build target *args: dart-build
    cd "$FLUTTER_EXAMPLE_WORKSPACE" && \
        flutter build {{target}} {{args}}

download-assets:
    cd "$FLUTTER_EXAMPLE_WORKSPACE"; \
    ./download_assets.sh

check-android-so:
    {{justfile_directory()}}/.github/scripts/check_android_so.sh "$FLUTTER_WORKSPACE"/android/src/main/jniLibs/

_override-flutter-self-deps $VERSION:
    #!/usr/bin/env bash
    set -eux -o pipefail
    cd "$FLUTTER_EXAMPLE_WORKSPACE"

    SED_CMD="sed"
    if [[ "{{os()}}" == "macos" ]]; then
        SED_CMD="gsed"
    fi

    # This will add changes to your repo which should never be committed.
    $SED_CMD -i s/dependency_overrides/HACK_hide_dependency_overrides/ ./pubspec.yaml
    $SED_CMD -i s/0.1.0+replace.with.version/${VERSION}/ ./pubspec.yaml


_dart-publish $WORKSPACE:
    #!/usr/bin/env bash
    set -eux -o pipefail
    cd "$WORKSPACE"

    SED_CMD="sed"
    # Default macOS doesn't support `sed -i`
    # Use `brew install gnu-sed`
    if [[ "{{os()}}" == "macos" ]]; then
        SED_CMD="gsed"
    fi

    # Dependency overrides are not allowed in published dart packages
    $SED_CMD -i s/dependency_overrides/HACK_hide_dependency_overrides/ ./pubspec.yaml

    # Use the branch name as metadata, replace invalid characters with "-".
    VERSION_METADATA="$(git rev-parse --abbrev-ref HEAD | sed s/[^0-9a-zA-Z-]/-/g )"
    if [[ "${VERSION_METADATA}" == "HEAD" ]]; then
        # use commit hash if we are in detached head mode
        VERSION_METADATA="$(git rev-parse HEAD)"
    fi

    # We use a timestamp as major version,
    # for now for our use case this is good enough and simple to do.
    TIMESTAMP="$(date +%y%m%d%k%M%S)"
    VERSION="0.${TIMESTAMP}.0+${VERSION_METADATA}"
    echo "Version: $VERSION"

    $SED_CMD -i s/0.1.0+replace.with.version/${VERSION}/ ./pubspec.yaml
    dart pub publish --force

# This should only be run by the CI
_ci-dart-publish:
    {{just_executable()}} _dart-publish "$FLUTTER_WORKSPACE"
    {{just_executable()}} _dart-publish "$DART_UTILS_WORKSPACE"
    {{just_executable()}} _dart-publish "$DART_WORKSPACE"

build-web-service:
    #!/usr/bin/env bash
    set -eux -o pipefail
    cd "$RUST_WORKSPACE"
    cargo build --release --bin web-api

alias d := dart-test
alias r := rust-test
alias t := test
alias pp := pre-push
