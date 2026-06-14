FROM rust:bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    musl-tools \
    pkg-config \
    libssl-dev \
    ca-certificates \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo
COPY src ./src
COPY x ./x
COPY migrations ./migrations
COPY static ./static

RUN rustup target add x86_64-unknown-linux-musl
RUN cargo x build --release --triple x86_64-unknown-linux-musl || \
    (echo "Build failed" && ls -la /app/target/x86_64-unknown-linux-musl/release && false)

FROM scratch AS runtime
ARG VERSION=dev
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /usr/share/zoneinfo /usr/share/zoneinfo
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/quest-log /usr/local/bin/quest-log
ENV PORT=3000
ENV QUEST_LOG_DATA_DIR=/data
ENV TZ=UTC
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/quest-log"]

LABEL org.opencontainers.image.source=https://github.com/bugabinga/quest-log
LABEL org.opencontainers.image.description="Gamified TODO-list for children"
LABEL org.opencontainers.image.licenses=MIT
LABEL org.opencontainers.image.version=$VERSION
