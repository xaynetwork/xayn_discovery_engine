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
    #!/usr/bin/env -S bash -eu -o pipefail
    export RUST_BACKTRACE=1
    cargo test --lib --bins --tests --locked
    cargo test --doc --locked

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

download-assets *args:
    #!/usr/bin/env -S bash -eu -o pipefail
    cd {{justfile_directory()}}/.github/scripts
    {{ if env_var_or_default("CI", "false") == "false" { "export AWS_PROFILE=\"S3BucketsDeveloperAccess-690046978283\"; echo AWS_PROFILE=$AWS_PROFILE;" } else { "" } }}
    ./download_assets.sh {{args}}

upload-assets *args:
    #!/usr/bin/env -S bash -eu -o pipefail
    {{ if env_var_or_default("CI", "false") == "false" { "export AWS_PROFILE=\"S3BucketsDeveloperAccess-690046978283\"; echo AWS_PROFILE=$AWS_PROFILE;" } else { "" } }}
    ./.github/scripts/prepare_data.sh {{args}}

build-service-args name target="default" features="":
    #!/usr/bin/env -S bash -eux -o pipefail
    if [[ -z "{{features}}" ]]; then
        features=""
    else
        features="--features {{features}}"
    fi
    if [[ "{{target}}" == "default" ]]; then
        target=""
    else
        target="--target {{target}}"
    fi
    echo "--release --bin {{name}} $target $features"

build-service name target="default" features="":
    #!/usr/bin/env -S bash -eux -o pipefail
    args=$(just build-service-args {{name}} {{target}} {{features}})
    if [[ "{{target}}" == "default" ]]; then
        cargo build $args
    else
        cross build $args
    fi

web-dev-up:
    #!/usr/bin/env -S bash -eu -o pipefail
    PROJECT=web-dev
    # -gt 1 because of the heading
    if [[ "$(docker ps --filter "label=com.docker.compose.project=$PROJECT" | wc -l)" -gt 1 ]]; then
        echo "web-dev composition is already running, SKIPPING STARTUP"
        exit 0
    fi
    if [[ "$(ls -l web-api/assets | grep 'assets/xaynia_v0002' | wc -l)" == "0" ]]; then
        rm "./web-api/assets" || :
        ln -s "./assets/xaynia_v0002" "./web-api/assets"
    fi
    export HOST_PORT_SCOPE=30
    docker-compose -p "$PROJECT" -f "./web-api/compose.db.yml" up --detach --remove-orphans --build

web-dev-down:
    #!/usr/bin/env -S bash -eu -o pipefail
    docker-compose -p web-dev -f "./web-api/compose.db.yml" down

build-service-image $CRATE_PATH $BIN $ASSET_DIR="":
    #!/usr/bin/env -S bash -eux -o pipefail
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
    docker build -f "$CRATE_PATH/Dockerfile" -t "xayn-$CRATE_PATH-$BIN" "$out"
    rm -rf "$out"

build-ci-image $IMAGE_NAME $IMAGE_TAG:
    #!/usr/bin/env -S bash -eux -o pipefail
    set -eux
    RUST_VERSION=$(perl -ne 'print $1 if /channel = \"(.*)\"/' {{justfile_directory()}}/rust-toolchain.toml)

    docker build \
      --build-arg rust_version="${RUST_VERSION}" \
      --build-arg just_version="${JUST_VERSION}" \
      --build-arg cargo_sort_version="${CARGO_SORT_VERSION}" \
      --build-arg spectral_cli_version="${SPECTRAL_CLI_VERSION}" \
      --build-arg ibm_openapi_ruleset_version="${IBM_OPENAPI_RULESET_VERSION}" \
      --build-arg validator_version="${VALIDATOR_VERSION}" \
      --tag "${IMAGE_NAME}:${IMAGE_TAG}" \
      - < .github/docker/Dockerfile.ci-image

compose-all-build $SMBERT="xaynia_v0002":
    #!/usr/bin/env -S bash -eux -o pipefail
    {{just_executable()}} build-service-image web-api personalization
    {{just_executable()}} build-service-image web-api ingestion "assets/$SMBERT"

compose-all-up *args:
    #!/usr/bin/env -S bash -eux -o pipefail
    PROJECT="compose-all"
    # -gt 1 because of the heading
    if [[ "$(docker ps --filter "label=com.docker.compose.project=$PROJECT" | wc -l)" -gt 1 ]]; then
        echo "compose-all composition is already running, can not continue in this case"
        exit 1
    fi
    export HOST_PORT_SCOPE=40
    docker-compose \
        -p "$PROJECT" \
        -f "./web-api/compose.db.yml" \
        -f "./web-api/compose.personalization.yml" \
        -f "./web-api/compose.ingestion.yml" \
        up \
        --detach --remove-orphans --build {{args}}

compose-all-down *args:
    #!/usr/bin/env -S bash -eux -o pipefail
    docker-compose \
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
    #!/usr/bin/env -S bash -eux -o pipefail
    # We need to call it once per file, if we pass in multiple files it will
    # have some bug where it does not report error correctly.
    for file in ls web-api/openapi/*.yaml; do
        spectral lint --verbose -F warn "$file"
    done

validate-migrations-unchanged cmp_ref:
    #!/usr/bin/env -S bash -eu -o pipefail
    if ! git rev-list "{{ cmp_ref }}".."{{ cmp_ref }}"; then
        git fetch --depth=1 "$(git remote get-url origin)" "{{ cmp_ref }}"
    	if ! git rev-list "{{ cmp_ref }}".."{{ cmp_ref }}"; then
          echo "ref: '{{cmp_ref}}' dosen't exits"
          exit 1
        fi
    fi

    changed_migrations=( $(\
        git diff --name-only "{{ cmp_ref }}" | \
        grep -E "^web-api-db-ctrl/postgres/.*" \
    ) ) || true

    if [ "${#changed_migrations[@]}" -gt 0 ]; then
        for migration in "${changed_migrations[@]}"; do
            echo "Migrations was changed ${migration}" >&2
        done
        exit 1
    else
        echo "OK - migrations unchanged"
    fi


print-just-env:
    export

mind-benchmark kind:
    cargo test --package xayn-web-api --release --lib \
        -- --nocapture --include-ignored --exact mind::run_{{kind}}_benchmark

tracing-flamegraph *args:
    #!/usr/bin/env -S bash -eu -o pipefail
    export XAYN_TEST_FLAME_LOG="${XAYN_TEST_FLAME_LOG:-info}"
    cargo test -- {{args}}
    for d in ./test-artifacts/*; do
        if [[ -e "$d/tracing.folded" && ! -e "$d/tracing.flamegraph.svg" ]]; then
            inferno-flamegraph "$d/tracing.folded" > "$d/tracing.flamegraph.svg"
            echo "Flamegraph stored at: $d/tracing.flamegraph.svg"
            inferno-flamegraph --flamechart  "$d/tracing.folded" > "$d/tracing.flamechart.svg"
            echo "Flamegraph stored at: $d/tracing.flamechart.svg"
        fi
    done

perf-flamegraph integration_test_bin:
    #!/usr/bin/env -S bash -eu -o pipefail
    export CARGO_PROFILE_BENCH_DEBUG=true
    OUT_DIR="./test-artifacts/{{integration_test_bin}}"
    mkdir -p "$OUT_DIR"
    cargo flamegraph -o "$OUT_DIR/flamegraph.svg"  --test {{integration_test_bin}}
    if [ -e "$OUT_DIR/perf.data" ]; then
        mv "$OUT_DIR/perf.data" "$OUT_DIR/perf.data.old"
    fi
    mv "perf.data" "$OUT_DIR/perf.data"

aws-login:
    #!/usr/bin/env bash
    {{ if env_var_or_default("CI", "false") == "false" { "export AWS_PROFILE=\"S3BucketsDeveloperAccess-690046978283\"" } else { "" } }}
    aws sso login

_test-project-root:
    #!/usr/bin/env -S bash -eu -o pipefail
    echo -n {{justfile_directory()}}

alias r := rust-test
alias t := test
alias pp := pre-push
