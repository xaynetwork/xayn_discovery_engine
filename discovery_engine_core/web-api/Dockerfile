FROM debian:bookworm-slim
ARG api
LABEL api="$api"
WORKDIR /app
#renamed as $api doesn't work in ENTRYPOINT
COPY ./"$api" ./server.bin
RUN chmod +x ./server.bin
ENTRYPOINT ["/app/server.bin"]
