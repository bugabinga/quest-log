# Architecture

## Bird's Eye View

Quest Log is a gamified todo list where users complete daily quests to earn XP
and unlock weekly rewards. It's a Rust web application with real-time updates
via Server-Sent Events (SSE).

## Code Map

### `x/`

Development task runner using cargo-xtask pattern. Provides `cargo x`
subcommands:

- `cargo x test` - Unit tests
- `cargo x verify` - All tests (including integration)
- `cargo x fmt` - Format Rust and JS
- `cargo x lint` - Format check + clippy + deno lint
- `cargo x check` - Full check (lint + verify + cargo check)
- `cargo x run [log_level] [cmd]` - Run application
- `cargo x watch` - Watch for changes
- `cargo x clean` - Clean build artifacts
- `cargo x container build [--release]` - Build container
- `cargo x container push [--release]` - Push to registry
- `cargo x bundle datastar [version]` - Bundle datastar JS
- `cargo x commit validate <file>` - Validate commit message

### `src/main.rs`

Application entry point. Sets up:

- Axum HTTP server
- Database connection (SQLite)
- Broadcast channel for SSE
- Graceful shutdown handling

### `src/handlers.rs`

HTTP request handlers:

- `quests` - Main page rendering
- `toggle_quest` - Mark quests complete/incomplete
- `navigate` - Day navigation (returns HTML fragments)
- `events` - SSE endpoint for real-time updates
- `claim_reward` - Weekly reward claims

### `src/database.rs`

SQLite database operations via sqlx. Handles quests, rewards, completion
tracking.

### `src/models.rs`

Data structures: Quest, Reward, QuestStats, etc.

### `src/time.rs`

Time/date utilities. Handles timezone-aware "today" calculation.

### `src/tui.rs`

Terminal UI for local quest management (alternative to web interface).

### `src/state.rs`

Application state: Database + broadcast channel.

### `src/ui/` (Maud templates)

Server-side HTML rendering using Maud:

- `base.rs` - Base template with common HTML scaffolding (head, nav, video
  modal)
- `quests.rs` - Main page template (uses base.rs)
- `error.rs` - Error page template (uses base.rs)
- `fragments/` - Reusable UI components (toggle, nav_buttons, etc.)

### `static/`

Static assets:

- `js/app.js` - Consolidated application JavaScript (SSE, UI interactions, death
  screen)
- `js/datastar.js` - Datastar library for reactive UI
- `style.css` - All styles including death screen overlay
- `images/`, `video/`, `fonts/`

## Architecture Invariants

- **Database**: SQLite, single file, in-memory for tests
- **No client-side state**: All state on server, client just renders what server
  sends
- **Timezone-aware**: Server calculates "today" based on user's timezone header
- **Graceful shutdown**: Server sends SSE events before shutting down, clients
  show death screen

## Cross-Cutting Concerns

### Real-time Updates

- Server uses broadcast channel to push updates to all connected clients
- Clients connect via SSE `/events` endpoint
- Datastar handles DOM morphing from server-sent HTML fragments

### Death Screen (Graceful Shutdown)

When server restarts/shuts down:

1. Server broadcasts `server-death` event
2. Server waits 500ms, then broadcasts `shutdown-complete`
3. Client shows overlay immediately (health polling detects server down)
4. Client polls `/health` every 2s until server returns
5. On server return, overlay hides, SSE reconnects

This two-phase approach ensures clients have time to receive the shutdown
message before the server dies.

### Error Handling

- `AppError` enum with Database/NotFound/ValidationError variants
- Error page rendered via Maud template
- Errors logged with tracing

### Testing

- Unit tests in `src/` (lib tests)
- Integration tests in `tests/`
- In-memory SQLite for test isolation

## JavaScript Structure

### `static/js/app.js`

Consolidated application JavaScript (all in one file for simplicity):

- Timezone header injection (fetch monkey-patch)
- Theme initialization (dark/light mode)
- SSE connection management
- Death screen overlay
- Video modal
- UI effects (notifications, confetti)
- Keyboard navigation
- Mutation observer for quest updates

### `static/js/datastar.js`

External library (~50KB) for reactive DOM updates. We use a small subset of its
features.

## Environment Variables

- `QUEST_LOG_DB` - Path to SQLite database (default: `~/.quest-log/questlog.db`)
- `QUEST_LOG_PORT` - HTTP port (default: 3000)
- `QUEST_LOG_TODAY` - Override "today" for testing (format: YYYY-MM-DD)
- `RUST_LOG` - Log level filter (default:
  `quest_log=debug,tokio=info,axum=warn`)

## Logging & Observability

### Tracing Setup

The project uses the `tracing` crate with `tracing-error` for structured
logging:

- **Tracing**: Auto-creates spans with timing for functions
- **ErrorLayer**: Captures span context with errors for debugging
- **Instrumented handlers**: All HTTP handlers have spans with emoji names
- **Instrumented DB methods**: Key database operations are traced

### Instrumented Functions

**Handlers** (with emoji span names):

- `📜 GET /` - Main page
- `📜 GET /day/:date` - Date-specific page
- `✨ toggle_quest` - Quest completion toggle
- `🧭 navigate` - Day navigation
- `📡 events` - SSE endpoint
- `🏆 claim_reward` - Reward claiming

**Database methods**:

- `📋 get_quests_for_day`
- `📋 get_quests_completion_status`
- `🎯 toggle_quest_completion`
- `🧮 calculate_weekly_exp`
- `📊 get_week_stats`
- `🎁 get_weekly_reward_status`
- `🏆 claim_reward_for_week`

### Log Levels

- `trace` - Most verbose, shows span enter/exit
- `debug` - Default for development, shows request handling
- `info` - General operational events
- `warn` - Unexpected but handled situations
- `error` - Failures

### Viewing Logs

```bash
# Debug (default)
cargo x run

# With trace logging
cargo x run trace

# Or manually
RUST_LOG=trace cargo run -- serve
```

### Client-Side Logging

JavaScript uses `console.log`/`console.error` with prefixes:

- `[SSE]` - Server-Sent Events
- `[Client]` - Client ID generation
- `[Fetch]` - Fetch wrapper
- `[Signals]` - Datastar signal patches
- `[Health]` - Health check polling
- `[JS]` - Uncaught errors

### Best Practices

- All database operations go through instrumented methods
- Errors include context via structured fields (`quest_id`, `date`, etc.)
- SSE events include correlation for debugging
- Tests use in-memory SQLite with debug logging disabled
