
# Make sure that some env variables are set for all jobs.
#FIXME: Consider using .env for this, but only if used also on CI
export RUST_WORKSPACE := env_var_or_default("RUST_WORKSPACE", "discovery_engine_core")
export DART_WORKSPACE := env_var_or_default("DART_WORKSPACE", "discovery_engine")

# Run just --list
default:
    @{{just_executable()}} --list

_codegen_workaround:
    cd "$RUST_WORKSPACE"; \
    cargo check --quiet 2>/dev/null || :

# Check rust related formatting
rust_check_fmt:
    cd "$RUST_WORKSPACE"; \
    cargo +nightly fmt --all -- --check ;\
    cargo sort --grouped --workspace --check

_rust_check_only: _codegen_workaround
    cd "$RUST_WORKSPACE"; \
    cargo clippy --all-targets --locked

_rust_check_doc_only: _codegen_workaround
    cd "$RUST_WORKSPACE"; \
    cargo doc --all-features --no-deps --document-private-items --locked

_rust_build_only: _codegen_workaround
    cd "$RUST_WORKSPACE"; \
    cargo build --locked

_rust_test_only: _codegen_workaround
    cd "$RUST_WORKSPACE"; \
    cargo test --locked

# Check rust related code (formatting, linting, type checks, etc.)
rust_check: rust_check_fmt _rust_check_only _rust_check_doc_only

# Run rust tests, including checks
rust_test: rust_check _rust_test_only

# Flutter iOS builds need the original C headers.
copy_ios_header:
    cp ./discovery_engine_core/bindings/include/*.h ./discovery_engine_flutter/ios/Classes/

# Generate rust to dart ffi (doesn't build rust)
rust_gen_ffi: _rust_check_only copy_ios_header

# Make sure dart dependencies are installed
dart_get_deps:
    cd "$DART_WORKSPACE"; \
    dart pub get

# Check dart formatting
dart_check_fmt:
    cd "$DART_WORKSPACE"; \
    dart format --output=none --set-exit-if-changed .

_dart_check_only: dart_get_deps
    cd "$DART_WORKSPACE"; \
    dart analyze --fatal-infos

_dart_check_doc_only: dart_get_deps
    cd "$DART_WORKSPACE"; \
    dart pub global run dartdoc:dartdoc --no-generate-docs --no-quiet

_dart_test_only:
    cd "$DART_WORKSPACE"; \
    dart test

_dart_gen_build_runner: dart_get_deps
    cd "$DART_WORKSPACE"; \
    dart run build_runner build --delete-conflicting-outputs

_dart_gen_genesis_only: dart_get_deps
    cd "$DART_WORKSPACE"; \
    dart run ffigen --config ffigen.yaml

_dart_gen_genesis_ext_only:
    cd "$RUST_WORKSPACE"; \
    "${CARGO_INSTALL_ROOT:-${CARGO_HOME:-$HOME/.cargo}}/bin/async-bindgen-gen-dart" \
        --ffi-class XaynDiscoveryEngineBindingsFfi \
        --genesis ../discovery_engine/lib/src/ffi/genesis.ffigen.dart

_dart_gen_ffi: rust_gen_ffi _dart_gen_genesis_only _dart_gen_genesis_ext_only

# Generate all dart code, including ffi code
dart_gen_code: _dart_gen_ffi _dart_gen_build_runner

# Check dart code for correctness (including formatting)
dart_check: dart_gen_code _dart_check_only _dart_check_doc_only

# Run dart tests (including ffi tests)
dart_test: dart_gen_code dart_check _rust_build_only _dart_test_only

# Run all tests
test: rust_test dart_test

# Check all code
check: rust_check dart_check

# Delete all generated files
codegen_clean:
    find . \( -name '*.g.dart' \
        -or -name '*.freezed.dart' \
        -or -name '*.ffigen.dart' \
        -or -name '*.ext.dart' \
    \) -exec rm '{}' \;
    -rm "$RUST_WORKSPACE"/bindings/include/*
    -rm "$RUST_WORKSPACE"/bindings/src/async_bindings/* || :

# Install async bindgen CLI utility.
install_async_bindgen:
    cargo install \
        --git https://github.com/xaynetwork/xayn_async_bindgen.git \
        async-bindgen-gen-dart \
        "$@"
