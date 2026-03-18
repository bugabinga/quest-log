# ADR-005: Core Technology Stack

## Status

Accepted

## Context

Quest Log needed a technology stack for a gamified todo list with real-time
updates. The application serves a single user (or small household) and must be
simple to deploy and maintain. Key requirements:

- **Single-binary deployment**: No external dependencies to manage
- **Real-time UI updates**: Users see quest completions instantly
- **Compile-time safety**: Catch errors before runtime
- **Low resource usage**: Runs on minimal hardware
- **Fast development**: Small codebase, quick iteration

## Decision Drivers

- **Must have**: Single-binary deployment, compile-time SQL/HTML validation
- **Must have**: Server-side state management (no client-side complexity)
- **Should have**: Low memory footprint, fast startup time
- **Should have**: Type-safe templates and queries
- **Could have**: Hot reload during development
- **Won't have**: Horizontal scaling (single-user app)
- **Won't have**: Complex client-side JavaScript frameworks

## Considered Options

### Web Framework

#### Option A: Axum

- **Pros**: Built on Tower/hyper, excellent async support, minimal overhead,
  great ecosystem integration
- **Cons**: Younger than alternatives, fewer middleware options

#### Option B: Actix-web

- **Pros**: Mature, battle-tested, extensive middleware ecosystem
- **Cons**: Heavier, more opinionated, larger binary size

#### Option C: Rocket

- **Pros**: Ergonomic API, compile-time route checking
- **Cons**: Slower release cycle, requires nightly for some features

### Database

#### Option A: SQLite

- **Pros**: Zero-config, single file, in-memory for tests, perfect for
  single-user apps
- **Cons**: No horizontal scaling, concurrent write limitations

#### Option B: PostgreSQL

- **Pros**: Powerful, excellent for concurrent access, rich feature set
- **Cons**: External dependency, overkill for single-user app, deployment
  complexity

#### Option C: sled / Redb

- **Pros**: Embedded, pure Rust, key-value simplicity
- **Cons**: No SQL, loses relational data modeling, less mature ecosystem

### HTML Templates

#### Option A: Maud

- **Pros**: Compile-time HTML validation, Rust-native syntax, zero runtime
  overhead
- **Cons**: Non-standard syntax, learning curve for HTML authors

#### Option B: Askama

- **Pros**: Jinja2-like syntax, compile-time checking, familiar to Python devs
- **Cons**: Template files separate from code, build-time processing

#### Option C: Tera

- **Pros**: Jinja2-compatible, runtime templates, large feature set
- **Cons**: Runtime errors possible, no compile-time validation

### Frontend Interactivity

#### Option A: Datastar (SSE-driven)

- **Pros**: Server-driven UI, minimal client JS, real-time via SSE, fits
  single-binary model
- **Cons**: Smaller community, newer library, tied to SSE architecture

#### Option B: HTMX

- **Pros**: Mature, large community, HTML-first approach
- **Cons**: Less built-in reactivity, requires more server endpoints

#### Option C: React/Vue (SPA)

- **Pros**: Rich ecosystem, component model, client-side routing
- **Cons**: Build complexity, client state management, loses single-binary
  simplicity

### Static Assets

#### Option A: static-serve (embedded)

- **Pros**: Assets in binary, single-file deployment, no external file serving
- **Cons**: Larger binary, must rebuild to update assets

#### Option B: External static files

- **Pros**: Smaller binary, update assets without rebuild
- **Cons**: Deployment complexity, file path management

## Decision

We chose the following stack:

| Layer         | Technology   | Version  |
| ------------- | ------------ | -------- |
| Language      | Rust         | 2024 ed. |
| Web Server    | Axum         | 0.8.x    |
| Database      | SQLite       | via sqlx |
| HTML          | Maud         | 0.27.x   |
| Interactivity | Datastar     | 1.0.x    |
| Assets        | static-serve | embedded |

## Rationale

### Rust 2024 Edition

Rust provides memory safety without garbage collection, excellent performance,
and a mature ecosystem. The 2024 edition brings improved async support and
ergonomics.

### Axum

Chosen for its minimal overhead and excellent integration with the Tower
ecosystem. Axum's extractors make request handling ergonomic, and its async
model fits our real-time SSE requirements perfectly.

### SQLite with sqlx

SQLite is ideal for a single-user application—zero configuration, single file
deployment, and in-memory mode for tests. sqlx provides compile-time SQL
verification, catching schema errors during development rather than at runtime.

### Maud

Maud's compile-time HTML validation catches typos and structure errors before
deployment. The macro-based syntax keeps templates in Rust code, enabling IDE
support and refactoring tools.

### Datastar

Datastar enables server-driven reactive UI via Server-Sent Events. The server
pushes HTML fragments, and Datastar morphs them into the DOM. This eliminates
client-side state management—everything lives on the server.

### Embedded Static Assets

Embedding assets in the binary ensures single-file deployment. No external file
paths to configure, no missing asset errors at runtime.

## Consequences

### Positive

- **Single-binary deployment**: Copy one file, set env vars, run
- **Compile-time safety**: SQL and HTML errors caught at build time
- **Minimal ops burden**: No database server to maintain, no CDN to configure
- **Fast cold starts**: Application starts in milliseconds
- **Low memory usage**: ~20-50MB typical, suitable for smallest VPS
- **Type safety end-to-end**: Rust, sqlx, and Maud form a safety net
- **Simple testing**: In-memory SQLite provides perfect isolation

### Negative

- **Asset updates require rebuild**: Changing images/CSS means recompiling
- **SQLite limitations**: Concurrent writes limited (not an issue for single
  user)
- **Learning curve**: Maud syntax unfamiliar to HTML developers
- **Datastar ecosystem**: Smaller community than HTMX/React
- **No horizontal scaling**: Architecture assumes single instance

### Risks

- **SQLite data loss**: Single file means single point of failure
  - _Mitigation_: Regular backups, WAL mode for durability
- **Datastar abandonment**: Small library could become unmaintained
  - _Mitigation_: Small subset of features used, could swap for HTMX
- **Rust compile times**: Large codebases slow to build
  - _Mitigation_: Keep codebase small, use incremental compilation

## Implementation Notes

### Key Patterns

1. **sqlx compile-time checks**: All queries use `query_as!` macro with
   `sqlx-data.json` for offline builds

2. **Maud templates**: Located in `src/ui/`, composed via `PreEscaped` for
   Datastar attributes

3. **SSE broadcasting**: `tokio::sync::broadcast` channel pushes updates to all
   connected clients

4. **State management**: `Arc<AppState>` with database pool and broadcast sender

5. **Graceful shutdown**: Server notifies clients before terminating (death
   screen pattern)

### File Structure

```
src/
├── main.rs          # Axum setup, routes, shutdown handling
├── database.rs      # sqlx queries with compile-time verification
├── models.rs        # Data structures matching DB schema
├── handlers/        # Route handlers returning HTML/SSE
└── ui/              # Maud templates
    └── fragments/   # Reusable UI components
```

### Development Workflow

```bash
# Run with auto-reload
cargo x watch

# Run tests (in-memory SQLite)
cargo x test

# Build for production
cargo x container build --release
```

## Related Decisions

- ADR-001: Use SQLX_OFFLINE for Container Release Builds - enables container
  builds without database connection
- ADR-002: Vendor All Web Resources in `static/` Directory - supports embedded
  asset pattern
- ADR-003: Use Tracing for Structured Logging and Observability - complements
  Rust ecosystem

## References

- [Axum Documentation](https://docs.rs/axum)
- [sqlx Compile-Time Verification](https://docs.rs/sqlx#compile-time-verification)
- [Maud Templates](https://maud.lambda.xyz/)
- [Datastar](https://data-star.dev/)
- [Tower Ecosystem](https://docs.rs/tower)
