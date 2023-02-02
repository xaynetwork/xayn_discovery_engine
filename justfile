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

# Fetches rust dependencies
rust-deps:
    #!/usr/bin/env bash
    set -eux -o pipefail
    cargo fetch {{ if env_var_or_default("CI", "false") == "true" { "--locked" } else { "" } }}

# Get/Update/Fetch/Install all dependencies
deps: rust-deps

# Formats rust (checks only on CI)
rust-fmt:
    #!/usr/bin/env bash
    set -eux -o pipefail
    cargo +nightly fmt --all -- {{ if env_var_or_default("CI", "false") == "true" { "--check" } else { "" } }};
    cargo sort --grouped --workspace {{ if env_var_or_default("CI", "false") == "true" { "--check --check-format" } else { "" } }}

# Formats all code (checks only on CI)
fmt: rust-fmt

# Checks rust code, fails on warnings on CI
rust-check:
    cargo clippy --all-targets --locked

# Checks all code, fails if there are any issues on CI
check: rust-check

# Checks if rust documentation can be build without issues
rust-check-doc:
    cargo doc --all-features --no-deps --document-private-items --locked

# Builds rust documentation
rust-doc *args:
    cargo doc --all-features --document-private-items --locked {{args}}

# Builds all documentation
doc: rust-doc

# Checks if all documentation can be build without issues
check-doc: rust-check-doc

# Builds rust
rust-build:
    cargo build --locked

# Builds all code
build: rust-build

# Tests rust
rust-test: download-assets
    #!/usr/bin/env bash
    set -eux -o pipefail
    cargo test --lib --bins --tests --quiet --locked
    cargo test --doc --quiet --locked

# Tests all code
test: rust-test

# Cleans up rusts build cache
rust-clean:
    cargo clean

# Removes all asset data
remove-assets:
    rm -rf ./assets/*

# Removes all local cached dependencies and generated files
clean: rust-clean

# Runs clean and removes assets
clean-fully: clean remove-assets

# Workaround to set env variable CI for all job dependencies
_pre-push: deps fmt check test

# Runs formatting, checks and test steps after deleting generated files.
pre-push $CI="true":
    @{{just_executable()}} _pre-push

download-assets:
    #!/usr/bin/env bash
    set -eux -o pipefail
    cd {{justfile_directory()}}/.github/scripts
    ./download_assets.sh

build-web-service:
    #!/usr/bin/env bash
    set -eux -o pipefail
    cargo build --release --bin personalization

build-ingestion-service:
    #!/usr/bin/env bash
    set -eux -o pipefail
    cargo build --release --bin ingestion

web-dev-up:
    #!/usr/bin/env -S bash -eux -o pipefail
    ociRunner="$(command -v podman || command -v docker)"
    compose="$(command -v podman-compose || command -v docker-compose)"
    PROJECT=web-dev
    # -gt 1 because of the heading
    if [[ "$("$ociRunner" ps --filter "label=com.docker.compose.project=$PROJECT" | wc -l)" -gt 1 ]]; then
        echo "web-dev composition is already running, SKIPPING STARTUP"
        exit 0
    fi
    if [[ "$(ls -l web-api/assets | grep 'assets/smbert_v0003' | wc -l)" == "0" ]]; then
        rm "./web-api/assets" || :
        ln -s "./assets/smbert_v0003" "./web-api/assets"
    fi
    export HOST_PORT_SCOPE=30
    "$compose" -p "$PROJECT" -f "./web-api/compose.db.yml" up --detach --remove-orphans --build

web-dev-down:
    #!/usr/bin/env -S bash -eux -o pipefail
    compose="$(command -v podman-compose || command -v docker-compose)"
    "$compose" -p web-dev -f "./web-api/compose.db.yml" down

build-service-image $CRATE_PATH $BIN $ASSET_DIR="":
    #!/usr/bin/env -S bash -eux -o pipefail
    ociBuilder="$(command -v podman || command -v docker)"
    out="$(mktemp -d -t xayn.web-api.compose.XXXX)"
    echo "Building in: $out"
    cargo install \
        --path "$CRATE_PATH" \
        --bin "$BIN" \
        --debug \
        --root "$out"
    # rename binary to the name the Dockerfile expects
    mv "$out/bin/$BIN" "$out/server.bin"
    rmdir "$out/bin"
    if [ -n "$ASSET_DIR" ]; then
        cp -R "$ASSET_DIR" "$out/assets"
    fi
    "$ociBuilder" build -f "$CRATE_PATH/Dockerfile" -t "xayn-$CRATE_PATH-$BIN" "$out"
    rm -rf "$out"

compose-all-build $SMBERT="smbert_v0003":
    #!/usr/bin/env -S bash -eux -o pipefail
    {{just_executable()}} build-service-image web-api personalization
    {{just_executable()}} build-service-image web-api ingestion "assets/$SMBERT"

compose-all-up *args:
    #!/usr/bin/env -S bash -eux -o pipefail
    ociRunner="$(command -v podman || command -v docker)"
    compose="$(command -v podman-compose || command -v docker-compose)"
    PROJECT="compose-all"
    # -gt 1 because of the heading
    if [[ "$("$ociRunner" ps --filter "label=com.docker.compose.project=$PROJECT" | wc -l)" -gt 1 ]]; then
        echo "compose-all composition is already running, can not continue in this case"
        exit 1
    fi
    export HOST_PORT_SCOPE=40
    "$compose" \
        -p "$PROJECT" \
        -f "./web-api/compose.db.yml" \
        -f "./web-api/compose.personalization.yml" \
        -f "./web-api/compose.ingestion.yml" \
        up \
        --detach --remove-orphans --build {{args}}

compose-all-down *args:
    #!/usr/bin/env -S bash -eux -o pipefail
    compose="$(command -v podman-compose || command -v docker-compose)"
    "$compose" \
        -p "compose-all" \
        -f "./web-api/compose.db.yml" \
        -f "./web-api/compose.personalization.yml" \
        -f "./web-api/compose.ingestion.yml" \
        down {{args}}

install-openapi-validator:
    #!/usr/bin/env -S bash -eux -o pipefail
    npm install -g \
      @stoplight/spectral-cli@${SPECTRAL_CLI_VERSION} \
      @ibm-cloud/openapi-ruleset@${IBM_OPENAPI_RULESET_VERSION} \
      validator@${VALIDATOR_VERSION}

validate-openapi:
    spectral lint --verbose -F warn web-api/openapi/*.yaml

print-just-env:
    export

mind-benchmark kind:
    cargo test --package xayn-web-api --release --lib \
        -- --nocapture --include-ignored --exact mind::run_{{kind}}_benchmark

_test-project-root:
    echo -n {{justfile_directory()}}

_test-generate-id:
    echo -n "t$(date +%y%m%d_%H%M%S)_$(printf "%04x" "$RANDOM")"

_test-create-dbs $TEST_ID:
    #!/usr/bin/env -S bash -eux -o pipefail
    psql -c "CREATE DATABASE ${TEST_ID};" postgresql://user:pw@localhost:3054/xayn 1>&2
    ./web-api/elastic-search/create_es_index.sh "http://localhost:3092/${TEST_ID}"

_test-drop-dbs $TEST_ID:
    #!/usr/bin/env -S bash -eux -o pipefail
    psql -c "DROP DATABASE ${TEST_ID};" postgresql://user:pw@localhost:3054/xayn 1>&2
    curl -f -X DELETE "http://localhost:3092/${TEST_ID}"

alias r := rust-test
alias t := test
alias pp := pre-push
