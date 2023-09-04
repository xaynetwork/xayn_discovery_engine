FROM debian:bookworm-20230502-slim

# rustls in aws
RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
        ca-certificates python3.11 pipenv \
        ; \
    update-ca-certificates

WORKDIR /app
COPY ./ ./

RUN pipenv install --deploy && pipenv run python -c 'import nltk; nltk.download("punkt")'

ENTRYPOINT ["/app/server.bin"]
