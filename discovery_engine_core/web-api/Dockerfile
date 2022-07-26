FROM debian:bookworm-slim

WORKDIR /service

COPY ./ ./

RUN chmod +x ./web-api
VOLUME /service/assets/data.json

CMD ["./web-api"]
