FROM debian:bookworm-20230502-slim

# rustls in aws
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    update-ca-certificates

WORKDIR /app
COPY ./ ./
ENTRYPOINT ["/app/server.bin"]
