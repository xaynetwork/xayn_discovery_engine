FROM debian:bookworm-slim
WORKDIR /app
ARG assets=""
COPY $assets ./assets
# include cargo-install metadata
ARG base="."
COPY $base/ ./
# RUN chmod +x /app/server.bin
ENTRYPOINT ["/app/server.bin"]
