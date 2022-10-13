FROM debian:bookworm-slim

WORKDIR /service

COPY ./ ./

RUN chmod +x ./web-api

EXPOSE 3000

CMD ["./web-api"]
