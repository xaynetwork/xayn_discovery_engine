FROM debian:bookworm-slim

WORKDIR /service

COPY ./ ./

RUN chmod +x ./backend

EXPOSE 8080

CMD ["./backend"]
