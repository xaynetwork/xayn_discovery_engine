
# Make sure that some env variables are set for all jobs.
#FIXME: Consider using .env for this, but only if used also on CI
export RUST_WORKSPACE := env_var_or_default("RUST_WORKSPACE", "discovery_engine_core")
export DART_WORKSPACE := env_var_or_default("DART_WORKSPACE", "discovery_engine")
export CARGO_INSTALL_ROOT := env_var_or_default("CARGO_INSTALL_ROOT", "cargo-installs")

# Run just --list
default:
    @{{just_executable()}} --list

# Get/update dart deps
dart_deps:
    cd "$DART_WORKSPACE"; \
    dart pub get

# Fetch rust dependencies
rust_deps:
    cd "$RUST_WORKSPACE"; \
    cargo fetch --locked

# Installs async-bindgen cli tool
install_async_bindgen *args:
    cargo install \
        --git https://github.com/xaynetwork/xayn_async_bindgen.git \
        {{args}} \
        async-bindgen-gen-dart \
        "$@" ;

# Get/Update/Fetch/Install all dependencies
deps: dart_deps rust_deps install_async_bindgen

# Format dart (checks only on CI)
dart_fmt:
    cd "$DART_WORKSPACE"; \
    dart format {{ if env_var_or_default("CI", "false") == "true" { "--output=none --set-exit-if-changed" } else { "" } }} .

# Format rust (checks only on CI)
rust_fmt:
    cd "$RUST_WORKSPACE"; \
    cargo +nightly fmt --all -- {{ if env_var_or_default("CI", "false") == "true" { "--check" } else { "" } }};\
    cargo sort --grouped --workspace {{ if env_var_or_default("CI", "false") == "true" { "--check" } else { "" } }}

# Format all code (checks only on CI)
fmt: rust_fmt dart_fmt

# Check dart code, fails on info on CI
dart_check: dart_build
    cd "$DART_WORKSPACE"; \
    dart analyze {{ if env_var_or_default("CI", "false") == "true" { "--fatal-infos" } else { "" } }}

_codegen_order_workaround:
    cd "$RUST_WORKSPACE"; \
    cargo check --quiet 2>/dev/null || :

# Check rust code, fails on warnings on CI
rust_check: _codegen_order_workaround
    cd "$RUST_WORKSPACE"; \
    cargo clippy --all-targets --locked #TODO DENY WARNINGS ON CI

# Check rust and dart code, fails if there are any issues on CI
check: rust_check dart_check

# Check if dart documentation can be build without issues
dart_check_doc:
    cd "$DART_WORKSPACE"; \
    dart pub global run dartdoc:dartdoc --no-generate-docs --no-quiet

# Check if dart documentation can be build without issues
rust_check_doc: _codegen_order_workaround
    cd "$RUST_WORKSPACE"; \
    cargo doc --all-features --no-deps --document-private-items --locked

# Check if documentation can be build without issues
check_doc: dart_check_doc rust_check_doc

_run_cbindgen: _codegen_order_workaround
    cd "$RUST_WORKSPACE"; \
    cargo check

_run_ffigen:
    cd "$DART_WORKSPACE"; \
    dart run ffigen --config ffigen.yaml

_run_async_bindgen:
    cd "$RUST_WORKSPACE"; \
    "{{justfile_directory()}}/$CARGO_INSTALL_ROOT/bin/async-bindgen-gen-dart" \
        --ffi-class XaynDiscoveryEngineBindingsFfi \
        --genesis ../"$DART_WORKSPACE"/lib/src/ffi/genesis.ffigen.dart

_run_build_runner:
    cd "$DART_WORKSPACE"; \
    dart run build_runner build --delete-conflicting-outputs

# Builds dart (runs all codegen steps)
dart_build: _run_cbindgen _run_ffigen _run_async_bindgen _run_build_runner

# Builds rust
rust_build:
    cd "$RUST_WORKSPACE"; \
    cargo build --locked

# Builds dart and rust
build: rust_build dart_build

# Tests dart (builds all necessary parts)
dart_test: rust_build dart_build
    cd "$DART_WORKSPACE"; \
    dart test

# Tests rust
rust_test:
    cd "$RUST_WORKSPACE"; \
    cargo test --locked

# Test dart and rust
test: rust_test dart_test

# Cleans up all generated files
clean_files:
    find . \( -name '*.g.dart' \
        -or -name '*.freezed.dart' \
        -or -name '*.ffigen.dart' \
        -or -name '*.ext.dart' \
    \) -exec rm '{}' \;
    -rm "$RUST_WORKSPACE"/bindings/include/*
    -rm "$RUST_WORKSPACE"/bindings/src/async_bindings/*

# Cleans up rusts build cache
rust_clean_deps:
    cd "$RUST_WORKSPACE"; \
    cargo clean

# Cleans up darts build cache
dart_clean_deps:
    find "$DART_WORKSPACE" -type d -name .dart_tool -prune -exec rm -r '{}' \;

# Remvoes all local cargo isntalls
remove_local_cargo_installs:
    rm -r "$CARGO_INSTALL_ROOT"

# Removes all local dependencies artifacts
clean_deps: rust_clean_deps dart_clean_deps remove_local_cargo_installs

# Removes all local cached dependencies and generated files
clean_fully: clean_files clean_deps

# Workaround to set env variable CI for all job dependencies
_pre_push: clean_files fmt check test

# Runs formatting check and test steps after deleting generated files.
pre_push $CI="true":
    @{{just_executable()}} _pre_push

alias cf := clean_files
alias d := dart_test
alias r := rust_test
alias t := test
alias pp := pre_push
