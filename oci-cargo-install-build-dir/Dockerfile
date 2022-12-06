FROM debian:bookworm-slim
WORKDIR /app
ARG base
COPY $base/bin/app.bin /app/app.bin
COPY $base/assets /app/assets
# include cargo-install metadata
COPY $base/.crates2.json /app/
RUN chmod +x ./app.bin
ENTRYPOINT ["/app/app.bin"]
