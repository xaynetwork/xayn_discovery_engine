FROM lukemathwalker/cargo-chef:0.1.62-rust-1.74-slim-bookworm AS chef
WORKDIR /app/project
# install rust version we use
COPY ./rust-toolchain.toml .
RUN cargo --version

FROM chef AS planner
RUN --mount=type=bind,target=. cargo chef prepare --recipe-path ../recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --target-dir ../target --recipe-path recipe.json --bin web-api
# Build application
COPY . .
RUN cargo build --release --target-dir ../target --bin web-api

FROM debian:12.2-slim AS runtime
WORKDIR /app
RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        # rustls in aws
        ca-certificates \
	python3.11 pipenv \
        ; \
    update-ca-certificates
COPY snippet-extractor/Pipfile snippet-extractor/Pipfile.lock .
COPY snippet-extractor/python_src/ python_src
RUN pipenv install --deploy &&\
    pipenv run python -c 'import nltk; nltk.download("punkt")'
ARG TARGETARCH
ARG MODEL_DIR="model directory is required"
ARG ORT_DIR="ort directory is required"
RUN --mount=type=bind,target=project set -eux; \
    # copy model \
    mkdir assets; \
    cp -r project/"$MODEL_DIR"/* assets/; \
    # copy ort lib \
    mkdir assets/lib; \
    case "$TARGETARCH" in \
        amd64) ORT_ARCH="x64" ;;\
        arm64) ORT_ARCH="aarch64" ;;\
    esac; \
    cp -r project/"$ORT_DIR"/linux_"$ORT_ARCH"/lib/* assets/lib

FROM runtime AS service
COPY --from=builder /app/target/release/web-api server.bin
ENTRYPOINT ["/app/server.bin"]

