FROM debian:bookworm-20230502-slim

RUN apt-get update
RUN apt-get install -y --no-install-recommends ca-certificates
RUN update-ca-certificates

WORKDIR /app
COPY ./ ./
ENTRYPOINT ["/app/server.bin"]
