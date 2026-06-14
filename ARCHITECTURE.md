# Architecture

Quest Log is a local-first Rust web app.
Server owns state; browser renders server HTML and receives Datastar/SSE patches.
SQLite stores quests, completions, rewards, settings, and history.

## Shape

- HTTP: Axum
- async runtime: Tokio
- DB: SQLite via sqlx migrations in `migrations/`
- UI: Maud templates in `src/ui/`
- reactivity: Datastar + server-sent events
- static assets: embedded from `static/`
- dev/build tasks: `cargo x` in `x/src/main.rs`

## Invariants

- Single deployed app binary.
- No client-owned business state.
- DB path comes from `QUEST_LOG_DATA_DIR`; container path is `/data`.
- Dev runtime state lives in `target/quest-log/`.
- Tests use isolated DBs.
- Public docs should link to source of truth, not duplicate route/module maps.

## Source of truth

- routes/router: `src/main.rs`
- config/env: `src/config.rs`
- DB API: `src/database/`
- domain types: `src/models.rs`
- UI templates: `src/ui/`
- handlers: `src/handlers/`
- command runner: `x/src/main.rs`
- decisions: `documentation/adrs/`
- specs: `documentation/specs/`
