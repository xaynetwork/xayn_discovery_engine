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
    cargo clippy --all-targets --locked {{ if env_var_or_default("CI", "false") == "true" { "--all-features" } else { "" } }}

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

build-backend-service:
    #!/usr/bin/env bash
    set -eux -o pipefail
    cargo build --release --bin backend

db-setup:
    #!/usr/bin/env bash
    set -eux -o pipefail
    ERROR=""
    (cargo sqlx --help 2>&1 | head -n1) || ERROR=$?
    if [[ "$ERROR" == "101" ]]; then
        echo 'You need to install sqlx-cli: `cargo install sqlx-cli`' >&2
        exit 101
    elif [[ -n "$ERROR" ]]; then
        echo '`cargo sqlx --help` failed in an unexpected way with exit code:' "$ERROR" >&2
        exit "$ERROR"
    fi
    export DATABASE_URL="sqlite:file://$(mktemp -d -t sqlx.discovery_engine.XXXX)/db.sqlite?mode=rwc"
    cargo sqlx database setup --source "core/src/storage/migrations"
    echo "DATABASE_URL=${DATABASE_URL}" >>.env.db.dev

db-migrate +ARGS:
    #!/usr/bin/env bash
    set -eux -o pipefail
    export $(cat .env.db.dev | xargs)
    cargo sqlx migrate --source "core/src/storage/migrations" {{ARGS}}

web-dev-up:
    #!/usr/bin/env -S bash -eux -o pipefail
    rm "./web-api/assets" || :
    ln -s "./assets/smbert_v0003" "./web-api/assets"
    compose="$(command -v podman-compose || command -v docker-compose)"
    $compose -f "./web-api/compose.yml" up --detach --remove-orphans

web-dev-down:
    #!/usr/bin/env -S bash -eux -o pipefail
    compose="$(command -v podman-compose || command -v docker-compose)"
    $compose -f "./web-api/compose.yml" down

install-openapi-validator:
    #!/usr/bin/env -S bash -eux -o pipefail
    npm install -g @stoplight/spectral-cli @ibm-cloud/openapi-ruleset validator  

validate-openapi:
    spectral lint --verbose -F warn web-api/openapi/*.yaml

print-just-env:
    export

alias r := rust-test
alias t := test
alias pp := pre-push
