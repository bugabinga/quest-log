# Quest Log Development Plan

## 🎮 Medieval Cyberpunk Steampunk Aesthetics

A retro gaming experience where arcane magic meets brass machinery—candlelight
flickers alongside neon tubes, glowing runes pulse through copper circuits. Glow
effects are subtle and ambient.

---

## ✅ Phase 13: Datastar Reactive Enhancement

- Migrated EXP counter to Datastar signals with `PatchSignals`
- Computed signals for `expTodayPercent` and `weekExpPercent`
- Weekly stats panel with progress bars
- Signal-based animation triggers

---

### 14.5 Containerization (Podman Quadlets)

**Goal:** Create containerized builds with systemd integration.

**Files to create:**

- `Containerfile` - Multi-stage build
- `quest-log.container` - Quadlet unit file
- `.containerignore` - Exclude files from build context

**Containerfile (multi-stage):**

```dockerfile
# Build stage
FROM rust:1.75-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY templates ./templates
COPY build.rs ./
RUN cargo build --release

# Runtime stage
FROM alpine:3.19
RUN apk add --no-cache ca-certificates
RUN adduser -D -u 1000 appuser
WORKDIR /app
COPY --from=builder /app/target/release/quest-log /app/quest-log
COPY migrations ./migrations
USER appuser
ENV PORT=3000
ENV RUST_LOG=info
EXPOSE 3000
ENTRYPOINT ["./quest-log"]
```

**quest-log.container (quadlet):**

```ini
[Unit]
Description=Quest Log Application
After=network.target

[Container]
Image=ghcr.io/anomalyco/quest-log:latest
Environment=QUEST_LOG_DATA_DIR=/var/lib/quest-log
Volume=/var/lib/quest-log:/var/lib/quest-log
PublishPort=3000:3000

[Install]
WantedBy=multi-user.target
```

**Registry:** `ghcr.io/<owner>/quest-log`

### 14.6 CI/CD Pipeline (GitHub Actions)

**Goal:** Automated build, test, and push to container registry.

**Files to create:**

- `.github/workflows/ci.yml`

**Workflow stages:**

1. **Check:** Format, clippy, unit tests
2. **Build:** Build container image
3. **Push:** Push to `ghcr.io` on main/tag

### 14.7 Health Endpoint

**Goal:** Simple health check for container orchestration.

**Files:**

- `src/main.rs` - Add `/health` route

**Implementation:**

```rust
async fn health() -> &'static str {
    "OK"
}
```

**Returns:** HTTP 200 with body "OK"

### 14.8 Documentation

**Files:**

- `README.md` - Add sections for:
  - Environment variables table
  - CLI commands
  - Deployment (Quadlets/Podman)
  - Container registry usage
- update `.env.example` with all supported environment variables
- polish user documentation with custom theme, matching quest log aesthetic
