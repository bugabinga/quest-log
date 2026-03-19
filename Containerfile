FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev musl-utils build-base pkgconfig openssl-dev ca-certificates tzdata

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY x ./x
COPY migrations ./migrations
COPY static ./static

RUN rustup target add x86_64-unknown-linux-musl
ENV RUSTFLAGS='-C target-feature=+crt-static'
RUN cargo build --release --target x86_64-unknown-linux-musl || \
    (echo "Build failed" && ls -la /app/target/x86_64-unknown-linux-musl/release && false)

FROM scratch AS runtime
ARG VERSION=dev
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /usr/share/zoneinfo /usr/share/zoneinfo
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/quest-log /usr/local/bin/quest-log
ENV PORT=3000
ENV TZ=UTC
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/quest-log"]

LABEL org.opencontainers.image.source=https://github.com/bugabinga/quest-log
LABEL org.opencontainers.image.description="Gamified TODO-list for children"
LABEL org.opencontainers.image.licenses=MIT
LABEL org.opencontainers.image.version=$VERSION
