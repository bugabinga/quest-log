# Quest Log Development Plan

## 🎮 Medieval Cyberpunk Steampunk Aesthetics

A retro gaming experience where arcane magic meets brass machinery—candlelight
flickers alongside neon tubes, glowing runes pulse through copper circuits. Glow
effects are subtle and ambient.

---

## ✅ Completed Phases

- **Phase 1-8**: Core infrastructure, UI, pixel aesthetics, glow effects
- **Phase 9**: Dark mode system (system preference detection only)
- **Phase 10**: Steampunk glow refinement (brass/copper/emerald/amethyst/ruby
  palette, flicker/gear-tick/rune-pulse animations)
- **Phase 11**: View transitions (quest morph, EXP counter, notifications)
- **Phase 11.5**: Full page transitions with SPA navigation (clean URLs, history
  API, slide animations)
- **Phase 12**: Mobile touch optimization (48px+ touch targets, touch/hover
  separation, portrait/landscape layouts, reduced motion support)

---

## ✅ Phase 12: Mobile Touch Optimization

- 48px+ touch targets for all interactive elements
- Touch vs hover separation with `@media (hover: hover)` and
  `@media (hover: none)`
- Portrait/landscape layout optimization
- Enhanced `prefers-reduced-motion` support with focus indicators

---

## ✅ Phase 13: Datastar Reactive Enhancement

- Migrated EXP counter to Datastar signals with `PatchSignals`
- Computed signals for `expTodayPercent` and `weekExpPercent`
- Weekly stats panel with progress bars
- Signal-based animation triggers

---

## 🏗️ Phase 14: 12-Factor Compliance

Adapt the application to the [Twelve-Factor App](https://12factor.net/)
methodology.

### Current Compliance Status

| Factor                 | Status | Notes                                 |
| ---------------------- | ------ | ------------------------------------- |
| I. Codebase            | ✅     | Single git repo                       |
| II. Dependencies       | ✅     | Cargo.toml declares all               |
| III. Config            | ⚠️     | Missing `.env.example`, no `RUST_LOG` |
| IV. Backing Services   | ✅     | SQLite via `QUEST_LOG_DATA_DIR`       |
| V. Build, release, run | ❌     | No containerization, no CI/CD         |
| VI. Processes          | ✅     | Stateless, DB-backed                  |
| VII. Port binding      | ✅     | `PORT` env var, self-contained        |
| VIII. Concurrency      | ✅     | Can scale horizontally                |
| IX. Disposability      | ❌     | No graceful shutdown                  |
| X. Dev/prod parity     | ❌     | No containerization                   |
| XI. Logs               | ❌     | `println!` instead of stdout stream   |
| XII. Admin processes   | ❌     | No CLI for migrations/admin           |

### 14.1 Environment Configuration

**Goal:** Document and standardize environment variables.

**Files:**

- `src/main.rs` - Remove `dotenvy::dotenv()` in release builds
- `.env.example` - **Create**

**Environment Variables:**

| Variable             | Description                                                  | Default           | Required |
| -------------------- | ------------------------------------------------------------ | ----------------- | -------- |
| `PORT`               | Server listen port                                           | `3000`            | No       |
| `QUEST_LOG_DATA_DIR` | Absolute path for database storage                           | Current directory | No       |
| `RUST_LOG`           | Tracing log level (e.g., `info`, `debug`, `quest_log=trace`) | `info`            | No       |

**Implementation:**

- `dotenvy::dotenv()` should only load `.env` in debug builds via
  `#[cfg(debug_assertions)]`
- Production relies purely on environment variables

### 14.2 Structured Logging

**Goal:** Replace `println!` with structured logging to stdout.

**Files:**

- `src/main.rs` - Add tracing setup, replace `println!`
- `Cargo.toml` - Add `tracing-subscriber` dependency

**Dependency:**

```toml
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

**Replace:**

- `println!("Initializing database...")` → `info!("Initializing database")`
- `println!("Database ready!")` → `info!("Database ready")`
- `println!("Server listening on http://{}", addr)` →
  `info!("Server listening on {}", addr)`

**Note:** Plain text output to stdout only. No JSON. Execution environment
handles log routing.

### 14.3 Graceful Shutdown

**Goal:** Handle SIGTERM/SIGINT for clean shutdown.

**Files:**

- `src/main.rs`

**Implementation:**

1. Use `tokio::signal::ctrl_c()` for SIGINT
2. Use `tokio::signal::unix::signal(SignalKind::terminate())` for SIGTERM
3. Wrap server in `tokio::select!` with shutdown signal
4. Add shutdown timeout (30 seconds)
5. Log shutdown progress
6. Replace `.unwrap()` with proper error handling

### 14.4 Admin CLI

**Goal:** Provide CLI for migrations and quest management.

**Files:**

- `src/cli.rs` - **Create**
- `src/main.rs` - Dispatch to CLI subcommands
- `Cargo.toml` - Add `clap` dependency

**Dependency:**

```toml
clap = { version = "4", features = ["derive"] }
```

**CLI Structure:**

```
quest-log                  # Start web server (default)
quest-log run              # Start web server (explicit)
quest-log migrate          # Run database migrations
quest-log quest list       # List all quests
quest-log quest create --title "..." --exp 10 --day 1
quest-log quest delete --id <ID>
quest-log quest complete --id <ID> --date YYYY-MM-DD
```

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

### 14.9 Justfile Commands

```just
# Run database migrations
migrate:
    cargo run -- migrate

# Build container image
build-container:
    podman build -t quest-log:latest -f Containerfile .

# Push to registry
push-container:
    podman push quest-log:latest ghcr.io/anomalyco/quest-log:latest

# Run container locally
run-container:
    podman run -it --rm -p 3000:3000 -e QUEST_LOG_DATA_DIR=/data -v quest-log-data:/data quest-log:latest
```

### Execution Order

1. 14.1 & 14.2 - Config & Logging (foundational)
2. 14.4 - Admin CLI (needed before containerization)
3. 14.7 - Health endpoint (simple, quick win)
4. 14.3 - Graceful shutdown
5. 14.5 - Containerization
6. 14.6 - CI/CD
7. 14.8 - Documentation
