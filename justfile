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
    cargo +"$RUST_NIGHTLY" fmt --all -- {{ if env_var_or_default("CI", "false") == "true" { "--check" } else { "" } }};
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

    # check if it's already _fully_ running
    running_services="$(
        $ociRunner ps --format json | \
        jq 'map(select(.Labels."com.docker.compose.project" == "web-api") | .Labels."com.docker.compose.service")'
    )"
    # Make sure we don't conflict with containerized ingestion or personalization services.
    # If we do detect this services we stop them and restart the dbs to create a clean state.
    RESTART=false
    if jq -e 'contains(["ingestion"])' <(echo "$running_services"); then
        RESTART=true
        $compose -f "./web-api/compose.ingestion.yml" down
    fi
    if jq -e 'contains(["personalization"])' <(echo "$running_services"); then
        RESTART=true
        $compose -f "./web-api/compose.personalization.yml" down
    fi
    if [ "$RESTART" = "true" ] || \
        jq -e 'contains(["elasticsearch","postgres"]) | not' <(echo "$running_services");
    then
        # stop any partial running services
        {{just_executable()}} web-dev-down 1>/dev/null 2>&1 || :
        # make sure the right assets are linked
        rm "./web-api/assets" || :
        ln -s "./assets/smbert_v0003" "./web-api/assets"
        # start all db services
        $compose -f "./web-api/compose.db.yml" up --detach --remove-orphans
    fi

web-dev-down:
    #!/usr/bin/env -S bash -eux -o pipefail
    compose="$(command -v podman-compose || command -v docker-compose)"
    $compose -f "./web-api/compose.db.yml" down

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
    compose="$(command -v podman-compose || command -v docker-compose)"
    "$compose" \
        -f web-api/compose.db.yml \
        -f web-api/compose.personalization.yml \
        -f web-api/compose.ingestion.yml \
        {{args}} \
        up

compose-all-down:
    #!/usr/bin/env -S bash -eux -o pipefail
    compose="$(command -v podman-compose || command -v docker-compose)"
    "$compose" \
        -f web-api/compose.db.yml \
        -f web-api/compose.personalization.yml \
        -f web-api/compose.ingestion.yml \
        down

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

project-root:
    echo -n {{justfile_directory()}}

alias r := rust-test
alias t := test
alias pp := pre-push
