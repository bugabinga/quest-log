FROM rust:1.70 AS builder
WORKDIR /usr/src/quest-log
COPY . .

# Build in release mode
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/quest-log/target/release/quest-log /usr/local/bin/quest-log
EXPOSE 3000
ENV PORT=3000
CMD ["/usr/local/bin/quest-log", "serve"]
