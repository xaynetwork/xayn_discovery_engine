#!/usr/bin/env -S bash -e

source .env
RUST_VERSION=$(perl -ne 'print $1 if /channel = \"(.*)\"/' rust-toolchain.toml)
TAG="$1"

docker build \
    --build-arg rust_version="${RUST_VERSION}" \
    --build-arg just_version="${JUST_VERSION}" \
    --build-arg cargo_sort_version="${CARGO_SORT_VERSION}" \
    --build-arg spectral_cli_version="${SPECTRAL_CLI_VERSION}" \
    --build-arg ibm_openapi_ruleset_version="${IBM_OPENAPI_RULESET_VERSION}" \
    --build-arg ibm_openapi_ruleset_utilities_version="${IBM_OPENAPI_RULESET_UTILITIES_VERSION}" \
    --build-arg validator_version="${VALIDATOR_VERSION}" \
    --build-arg redocly_cli_version="${REDOCLY_CLI_VERSION}" \
    --tag "${TAG}" \
    - < .github/docker/Dockerfile.ci-image
