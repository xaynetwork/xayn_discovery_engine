FROM debian:bookworm-slim
ARG api
LABEL api="$api"
WORKDIR /app
# copy everything in the cwd (bin and assets)
COPY ./ ./
# renamed as $api doesn't work in ENTRYPOINT
RUN mv ./"$api" ./server.bin
RUN chmod +x ./server.bin
ENTRYPOINT ["/app/server.bin"]
