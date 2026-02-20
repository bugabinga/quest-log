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

| Factor                 | Status      | Notes                                               |
| ---------------------- | ----------- | --------------------------------------------------- |
| I. Codebase            | ✅          | Single git repo                                     |
| II. Dependencies       | ✅          | Cargo.toml declares all                             |
| III. Config            | ✅          | .env.example, RUST_LOG supported                    |
| IV. Backing Services   | ✅          | SQLite via `QUEST_LOG_DATA_DIR`                     |
| V. Build, release, run | in_progress | Containerization planned                            |
| VI. Processes          | ✅          | Stateless, DB-backed                                |
| VII. Port binding      | ✅          | `PORT` env var, self-contained                      |
| VIII. Concurrency      | ✅          | Can scale horizontally                              |
| IX. Disposability      | ✅          | Graceful shutdown implemented                       |
| X. Dev/prod parity     | in_progress | No containerization                                 |
| XI. Logs               | ✅          | Structured logging with emojis                      |
| XII. Admin processes   | in_progress | Interactive TUI replaces CLI; no scripted admin CLI |

### 14.1 Environment Configuration ✅

**Done:** Implemented in main.rs with #[cfg(debug_assertions)] guard, created
.env.example

### 14.2 Structured Logging ✅

**Done:** Added tracing with fun emoji logs, debug mode has clean output

### 14.3 Graceful Shutdown

**Goal:** Handle SIGTERM/SIGINT for clean shutdown.

**Files:**

- `src/main.rs`

**Status:** ✅ Implemented in `src/main.rs`. The server installs
`tokio::signal::ctrl_c()` and a SIGTERM handler on unix, uses a oneshot bridge
with `with_graceful_shutdown`, and waits up to 30s for in-flight requests to
finish. See `Database::new()` for migration behavior and `systemd` module
integration for readiness/watchdog notifications.

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

**Status:** Interactive TUI implemented. `src/cli.rs` dispatches to `src/tui.rs`
for interactive admin operations. A non-interactive `migrate-only` subcommand
was added to apply embedded migrations and exit (useful for CI/release hooks).

**Dependency:**

```toml
clap = { version = "4", features = ["derive"] }
```

**CLI Structure:**

```
quest-log                  # Start web server (default)
quest-log run              # Start web server (explicit)
quest-log migrate          # Run database migrations
quest-log quest list       # List quests [--day 1]
quest-log quest create     # --title, --exp, --day
quest-log quest delete     # --id
quest-log quest complete   # --id, --date
quest-log reward list      # List rewards
quest-log reward create   # --title, --required-exp
quest-log reward delete   # --id
quest-log settings show   # Show current settings
quest-log settings set    # --weekly-goal
```

**Notes:**

- CLI directly accesses SQLite database file (WAL mode handles concurrency)
- No IPC with running server needed - CLI changes reflected on next page refresh
- Quest `--day` defaults to current day of week
- Reward `--required-exp` (or `--exp`) specifies EXP needed to claim
- Settings `--weekly-goal` sets the weekly EXP target (default: 100)

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
