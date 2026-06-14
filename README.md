# Quest Log

Small local web app for children: daily quests earn EXP; weekly goals unlock rewards.
Parents manage quests, rewards, and settings in the editor UI.

## Status

Active personal app. Container install is intended path, but public release is TODO.

## Develop

Use `cargo x`. Do not call project build/test commands directly.

```bash
cargo x build   # build binary
cargo x check   # fast typecheck
cargo x fmt     # format
cargo x lint    # quality gate
cargo x test    # unit tests
cargo x verify  # all tests
cargo x serve   # background dev server
cargo x kill    # stop dev server
```

Open <http://localhost:3000>.

Dev server state lives under `target/quest-log/` unless `QUEST_LOG_DATA_DIR` is set.

## Install

TODO: publish container image.

Intended container contract:

- port: `3000`
- data dir: `/data`
- persistent volume mounted at `/data`
- database file: `/data/quests.db`

Local image commands live in `cargo x container ...`.

## Runtime config

See `.env.example` and `src/config.rs` for current env vars.

## Current project facts

Do not trust copied module/route lists in docs. Inspect sources:

- commands: `x/src/main.rs`
- routes: `src/main.rs`
- config: `src/config.rs`
- architecture notes: `ARCHITECTURE.md`
- decisions: `documentation/adrs/`
- specs: `documentation/specs/`
