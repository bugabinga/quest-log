# Quest Log Agent Instructions

## Essential Directives

**ALWAYS** use `cargo x` for development tasks. Commands like `cargo x test`,
`cargo x lint`, `cargo x check` should be used instead of calling cargo
directly. All build/deploy/maintenance commands MUST run through `cargo x`.
Direct `cargo`, `rustc`, or other tool commands are forbidden. If a command is
missing from `x/src/main.rs`, add it there instead.

Keep comments accurate, descriptive, and up-to-date. Update comments when
modifying code. Accurate comments aid comprehension; inaccurate or outdated
comments hinder it.

## VERBOTEN

Do not do these things:

- never use `#[allow]` without human approval

## Project Structure

### Source code

src/ ├── main.rs # Server entry point ├── lib.rs # Core business logic ├──
database.rs # SQLite operations ├── models.rs # Data structures ├── handlers/ #
HTTP route handlers │ ├── quests.rs # Main UI endpoints │ ├── parent.rs #
Settings endpoints │ └── stats.rs # Highscore endpoints ├── auth.rs #
Authentication ├── state.rs # App state management ├── time.rs # Time utilities
├── systemd.rs # Systemd integration └── ui/ # UI fragments └── fragments/ #
HTML components

### Build tool

x/ # Custom CLI for development tasks (cargo x ...)

### Test files

tests/ # Integration tests

### Configuration

Containerfile # Container build definition migrations/ # Database migrations
static/ # Static assets (images, JS, CSS) documentation/ # Documentation and
ADRs

## Commands

| Command             | Description                              |
| ------------------- | ---------------------------------------- |
| `cargo x test`      | Run unit tests                           |
| `cargo x verify`    | Run all tests (integration)              |
| `cargo x fmt`       | Format Rust and JS code                  |
| `cargo x lint`      | Check formatting, clippy, and lint       |
| `cargo x check`     | Full check (lint + verify + cargo check) |
| `cargo x run`       | Run the application (foreground, blocks) |
| `cargo x serve`     | Run the application (background)         |
| `cargo x kill`      | Kill the background server               |
| `cargo x watch`     | Watch for changes and rebuild            |
| `cargo x clean`     | Clean build artifacts and database       |
| `cargo x container` | Container operations (build, push, etc.) |
| `cargo x bundle`    | Bundle datastar from CDN                 |
| `cargo x assets`    | Process assets (favicons, icons)         |
| `cargo x commit`    | Validate commit message format           |

## Server Management

- `cargo x run` - Run server in foreground. **Blocks** the terminal. Use only
  when debugging directly or needing live log output visible.
- `cargo x serve` - Run server in background. **Non-blocking**. Preferred for
  agent workflows. Automatically kills any existing server first.
- `cargo x kill` - Stop the background server. Idempotent (safe to call even if
  no server is running).

**For agents:** Always use `cargo x serve` instead of `cargo x run`. The
blocking behavior of `cargo x run` prevents the agent from executing further
actions in the same session.

**Note:** Test commands (`cargo x test`, `cargo x verify`, `cargo x check`)
automatically enable the `test-utils` feature flag, which provides test
utilities like date override functions. Developers should always use `cargo x`
commands instead of running `cargo` directly.

## Code Standards

### Rust

- snake_case functions/variables, PascalCase structs/enums/traits
- SCREAMING_SNAKE_CASE constants
- `Result<T, E>` for errors, avoid `unwrap()`/`expect()`
- `async fn` with tokio, `Arc<Mutex<T>>` for shared state
- Group imports: std → external → local

### HTML/CSS

- Semantic HTML, BEM naming, flexbox/grid layouts, mobile-first
- Never hardcode colors - derive everything from 3 base colors
- Use data attributes for datastar

### Datastar + Maud + Axum Integration

When building reactive UI features, follow these patterns:

- Use Maud templates in `src/ui/fragments/` for type-safe HTML components
- Return SSE streams with `PatchElements` and `PatchSignals` from handlers
- Use `ReadSignals` extractor to get client-sent data
- Enable view transitions for smooth animations
- Broadcast updates to all connected clients via `state.bcast`

**For guidance on Datastar/Maud/Axum integration, load the `datastar-maud-axum`
skill.**

## Environment

Required:

- `QUEST_LOG_DATA_DIR` - Directory path for storing the SQLite database file
  (must be absolute path, default: current directory)
- `PORT` - Server port (default: `3000`)

Setup:

- Database migrations are embedded in the binary and applied automatically at
  startup
- For explicit migration runs: `cargo x container migrate`

## Gotchas

- Systemd integration requires `--features systemd` flag (Linux only)
- Container builds require clean git state for release builds
- Datastar bundling requires deno to be installed
- Tests must run with SQLite in-memory databases for isolation
- HTML templates use Maud for compile-time validation
- Environment variables must be set before running the application

## External References

For architecture details: @ARCHITECTURE.md For ADRs: @documentation/adrs/ For
project overview: @README.md

## Commit Guidelines

Present tense, focused single changes, reference issues.

## Security

Validate inputs, parameterized queries, sanitize HTML output.

## Documentation

rustdoc all public APIs. See ARCHITECTURE.md for architecture details,
logging/tracing setup, and ADRs in documentation/adrs/.
