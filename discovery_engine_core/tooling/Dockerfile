FROM debian:bookworm-slim

WORKDIR /service

COPY ./ ./

RUN chmod +x ./backend

EXPOSE 3000

CMD ["./backend"]