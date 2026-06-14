# Quest Log Agent Rules

## Style

Simple. Minimal. Brutal.
No legacy/compat hedging unless user asks.
Delete fluff. Keep docs short.

## Commands

Use `cargo x` for every project build/test/dev task.
No direct `cargo`, `rustc`, `deno`, `podman`, etc. for project workflows.
If a task has no `cargo x` command, add one in `x/src/main.rs`.

Command contract:

- `cargo x build` = build binary
- `cargo x check` = fast typecheck
- `cargo x fmt` = format
- `cargo x lint` = quality gate
- `cargo x test` = unit tests
- `cargo x verify` = all tests
- `cargo x serve` = background dev server
- `cargo x kill` = stop dev server

Current command truth: `x/src/main.rs`.

## Non-normal project facts

- App dependency versions use `"*"` intentionally. Do not “fix”.
- Dev runtime state belongs in `target/quest-log/`.
- Container data belongs in `/data`.
- Maud + Datastar + Axum drive UI. Load `datastar-maud-axum` skill before UI/reactive changes.
- Do not copy/cache repo maps, route lists, or command lists into docs. Link to source of truth.

## Code

- No `#[allow]` without human approval.
- Keep comments accurate or delete them.
- Prefer small focused files/functions.
- Use tests for behavior changes.

## Source of truth

- routes: `src/main.rs`
- config: `src/config.rs`
- commands: `x/src/main.rs`
- architecture: `ARCHITECTURE.md`
- ADRs/specs: `documentation/`
