FROM debian:bookworm-slim
WORKDIR /app
ARG base="."
# include cargo-install metadata
COPY $base/ ./
RUN chmod +x /app/server.bin
ENTRYPOINT ["/app/server.bin"]
