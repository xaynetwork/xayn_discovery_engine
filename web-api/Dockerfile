FROM debian:bookworm-slim
WORKDIR /app
# copy everything in the cwd (bin and assets)
COPY ./ ./
RUN chmod +x ./server.bin
ENTRYPOINT ["/app/server.bin"]
