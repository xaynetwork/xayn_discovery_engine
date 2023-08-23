FROM debian:bookworm-20230502-slim

# rustls in aws
RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        ca-certificates python3.11 libpython3.11 pipenv \
        ; \
    update-ca-certificates

WORKDIR /app
COPY ./ ./

RUN pipenv install --system --deploy

ENTRYPOINT ["/app/server.bin"]
