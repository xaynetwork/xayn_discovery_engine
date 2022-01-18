
export RUST_WORKSPACE := env_var_or_default("RUST_WORKSPACE", "discovery_engine_core")
export DART_WORKSPACE := env_var_or_default("DART_WORKSPACE", "discovery_engine")

default:
    @{{just_executable()}} --list

_codegen_workaround:
    cd "$RUST_WORKSPACE"; \
    cargo check --quiet 2>/dev/null || :

rust_check_fmt:
    cd "$RUST_WORKSPACE"; \
    cargo +nightly fmt --all -- --check ;\
    cargo sort --grouped --workspace --check

_rust_check_only: _codegen_workaround
    cd "$RUST_WORKSPACE"; \
    cargo clippy --all-targets --locked

_rust_build_only: _codegen_workaround
    cd "$RUST_WORKSPACE"; \
    cargo build --locked

_rust_test_only: _codegen_workaround
    cd "$RUST_WORKSPACE"; \
    cargo test --locked


rust_check: rust_check_fmt _rust_check_only

rust_test: rust_check _rust_test_only

copy_ios_header:
    cp ./discovery_engine_core/bindings/include/*.h ./discovery_engine_flutter/ios/Classes/

rust_gen_ffi: _rust_check_only copy_ios_header

dart_get_deps:
    cd "$DART_WORKSPACE"; \
    dart pub get

dart_check_fmt:
    cd "$DART_WORKSPACE"; \
    dart format --output=none --set-exit-if-changed .

_dart_check_only: dart_get_deps
    cd "$DART_WORKSPACE"; \
    dart analyze --fatal-infos

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

dart_gen_code: _dart_gen_ffi _dart_gen_build_runner

dart_check: dart_gen_code _dart_check_only

dart_test: dart_gen_code dart_check _rust_build_only _dart_test_only

test: rust_test dart_test

check: rust_check dart_check

codegen_clean:
    find . \( -name '*.g.dart' \
        -or -name '*.freezed.dart' \
        -or -name '*.ffigen.dart' \
        -or -name '*.ext.dart' \
    \) -exec rm '{}' \;
    -rm "$RUST_WORKSPACE"/bindings/include/*
    -rm "$RUST_WORKSPACE"/bindings/src/async_bindings/* || :

install_async_bindgen:
    cargo install \
        --git https://github.com/xaynetwork/xayn_async_bindgen.git \
        async-bindgen-gen-dart \
        "$@"
