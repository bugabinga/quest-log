# ADR-013: PGO-Optimized Container Builds

## Status

Proposed

## Context

Release containers are normal optimized Rust builds. PGO might improve hot paths:
SQLite queries, Maud rendering, and Datastar/SSE flows.

## Decision

If PGO is implemented, it must be driven through `cargo x` commands only:

- build binaries with `cargo x build --release`
- exercise browser flows with `cargo x browser`
- build images with `cargo x container build`

No separate project workflow commands.

## Consequences

- Better profile realism than unit-test-only PGO.
- Slower, more complex image builds.
- Not worth implementing until measured container performance needs it.
