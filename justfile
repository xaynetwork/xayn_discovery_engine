
# Make sure that some env variables are set for all jobs.
#FIXME: Consider using .env for this, but only if used also on CI
export RUST_WORKSPACE := env_var_or_default("RUST_WORKSPACE", "discovery_engine_core")
export DART_WORKSPACE := env_var_or_default("DART_WORKSPACE", "discovery_engine")
export CARGO_INSTALL_ROOT := env_var_or_default("CARGO_INSTALL_ROOT", "cargo-installs")

# Run just --list
default:
    @{{just_executable()}} --list

# Get/update dart deps
dart-deps:
    cd "$DART_WORKSPACE"; \
    dart pub get

# Fetch rust dependencies
rust-deps:
    cd "$RUST_WORKSPACE"; \
    cargo fetch {{ if env_var_or_default("CI", "false") == "true" { "--locked" } else { "" } }}

# Installs async-bindgen cli tool
install-async-bindgen *args:
    cargo install \
        --git https://github.com/xaynetwork/xayn_async_bindgen.git \
        {{args}} \
        async-bindgen-gen-dart \
        "$@" ;

# Get/Update/Fetch/Install all dependencies
deps: dart-deps rust-deps install-async-bindgen

# Format dart (checks only on CI)
dart-fmt:
    cd "$DART_WORKSPACE"; \
    dart format {{ if env_var_or_default("CI", "false") == "true" { "--output=none --set-exit-if-changed" } else { "" } }} .

# Format rust (checks only on CI)
rust-fmt:
    cd "$RUST_WORKSPACE"; \
    cargo +nightly fmt --all -- {{ if env_var_or_default("CI", "false") == "true" { "--check" } else { "" } }};\
    cargo sort --grouped --workspace {{ if env_var_or_default("CI", "false") == "true" { "--check" } else { "" } }}

# Format all code (checks only on CI)
fmt: rust-fmt dart-fmt

# Check dart code, fails on info on CI
dart-check: dart-build
    cd "$DART_WORKSPACE"; \
    dart analyze {{ if env_var_or_default("CI", "false") == "true" { "--fatal-infos" } else { "" } }}

_codegen-order-workaround:
    cd "$RUST_WORKSPACE"; \
    cargo check --quiet 2>/dev/null || :

# Check rust code, fails on warnings on CI
rust-check: _codegen-order-workaround
    cd "$RUST_WORKSPACE"; \
    cargo clippy --all-targets --locked #TODO DENY WARNINGS ON CI

# Check rust and dart code, fails if there are any issues on CI
check: rust-check dart-check

# Check if dart documentation can be build without issues
dart-check-doc:
    cd "$DART_WORKSPACE"; \
    dart pub global run dartdoc:dartdoc --no-generate-docs --no-quiet

# Check if dart documentation can be build without issues
rust-check_doc: _codegen-order-workaround
    cd "$RUST_WORKSPACE"; \
    cargo doc --all-features --no-deps --document-private-items --locked

# Check if documentation can be build without issues
check_doc: dart-check-doc rust-check_doc

_run-cbindgen: _codegen-order-workaround
    cd "$RUST_WORKSPACE"; \
    cargo check

_run-ffigen:
    cd "$DART_WORKSPACE"; \
    dart run ffigen --config ffigen.yaml

_run-async-bindgen:
    cd "$RUST_WORKSPACE"; \
    "{{justfile_directory()}}/$CARGO_INSTALL_ROOT/bin/async-bindgen-gen-dart" \
        --ffi-class XaynDiscoveryEngineBindingsFfi \
        --genesis ../"$DART_WORKSPACE"/lib/src/ffi/genesis.ffigen.dart

_run-build-runner:
    cd "$DART_WORKSPACE"; \
    dart run build_runner build --delete-conflicting-outputs

# Builds dart (runs all codegen steps)
dart-build: _run-cbindgen _run-ffigen _run-async-bindgen _run-build-runner

# Builds rust
rust-build:
    cd "$RUST_WORKSPACE"; \
    cargo build --locked

# Builds dart and rust
build: rust-build dart-build

# Tests dart (builds all necessary parts)
dart-test: rust-build dart-build
    cd "$DART_WORKSPACE"; \
    dart test

# Tests rust
rust-test:
    cd "$RUST_WORKSPACE"; \
    cargo test --locked

# Test dart and rust
test: rust-test dart-test

# Cleans up all generated files
clean-files:
    find . \( -name '*.g.dart' \
        -or -name '*.freezed.dart' \
        -or -name '*.ffigen.dart' \
        -or -name '*.ext.dart' \
    \) -exec rm '{}' \;
    -rm "$RUST_WORKSPACE"/bindings/include/*
    -rm "$RUST_WORKSPACE"/bindings/src/async_bindings/*

# Cleans up rusts build cache
rust-clean-deps:
    cd "$RUST_WORKSPACE"; \
    cargo clean

# Cleans up darts build cache
dart-clean-deps:
    find "$DART_WORKSPACE" -type d -name .dart_tool -prune -exec rm -r '{}' \;

# Remvoes all local cargo isntalls
remove-local-cargo-installs:
    rm -r "$CARGO_INSTALL_ROOT"

# Removes all local dependencies artifacts
clean-deps: rust-clean-deps dart-clean-deps remove-local-cargo-installs

# Removes all local cached dependencies and generated files
clean-fully: clean-files clean-deps

# Workaround to set env variable CI for all job dependencies
_pre-push: clean-files fmt check test

# Runs formatting check and test steps after deleting generated files.
pre-push $CI="true":
    @{{just_executable()}} _pre-push

alias cf := clean-files
alias d := dart-test
alias r := rust-test
alias t := test
alias pp := pre-push
