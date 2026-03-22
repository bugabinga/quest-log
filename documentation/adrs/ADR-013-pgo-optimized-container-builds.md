# ADR-013: PGO-Optimized Container Builds

## Status

Proposed

## Context

The current container build process uses a standard release build without
Profile-Guided Optimization (PGO). The quest-log application has performance-
critical paths that could benefit from PGO:

- **SQLite queries**: Database operations for quests, rewards, and user data
- **HTML rendering**: Maud template compilation and rendering
- **SSE handling**: Server-Sent Events for real-time UI updates via Datastar

Current Containerfile is a simple two-stage build:

```
rust:alpine (builder) → scratch (runtime)
```

This produces a working binary but doesn't leverage runtime profiling data that
could inform compiler optimizations for the actual workload patterns.

## Decision Drivers

- **Must have**: Automated PGO in container build (no manual steps)
- **Must have**: Reproducible builds across environments
- **Should have**: Realistic workload for profile collection
- **Should have**: Measurable performance improvement (5%+ expected)
- **Could have**: Fast build times (secondary to optimization quality)
- **Won't have**: Local developer PGO workflow (too complex)

## Considered Options

### Option A: Local PGO Profiles

Build instrumented binary locally, run representative workload, copy `.profdata`
to container build.

- **Pros**: Simple Containerfile, fast container builds, control over workload
- **Cons**: Manual process, profiles may not match production, extra files to
  manage, developer friction

### Option B: Unit Tests Only for PGO

Run unit tests (not E2E) during container build for profile collection.

- **Pros**: Faster profile collection (seconds vs minutes), no browser required
- **Cons**: Unit tests don't exercise SSE, HTML rendering, or realistic request
  patterns; lower quality profiles

### Option C: Full E2E PGO (Chosen)

Run complete E2E test suite with headless browser during container build for
profile collection.

- **Pros**: Most representative workload, fully automated, tests real code paths
  (SQLite, HTML, SSE)
- **Cons**: Complex Containerfile, longer builds (3-4x), requires Chrome+xvfb in
  builder stage

## Decision

Replace the current Containerfile with a PGO-enabled multi-stage build:

```
┌─────────────────────────────────────────────────────────────┐
│ base (rust:slim)                                            │
│ • llvm-tools-preview                                        │
│ • deno (for E2E tests)                                      │
│ • chromium + xvfb (headless browser)                        │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ instrumented                                                │
│ • RUSTFLAGS='-Cprofile-generate=/profiling'                 │
│ • cargo build --release                                     │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ profiler                                                    │
│ • Start instrumented binary                                 │
│ • Run E2E tests: deno task test:e2e                         │
│ • Shutdown server cleanly                                   │
│ • llvm-profdata merge → merged.profdata                     │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ optimizer                                                   │
│ • RUSTFLAGS='-Cprofile-use=/profiling/merged.profdata'      │
│ • cargo build --release                                     │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ runtime (scratch)                                           │
│ • Optimized binary only                                     │
│ • CA certificates, timezone data                            │
└─────────────────────────────────────────────────────────────┘
```

### Files to Create/Modify

1. **Containerfile** - Replace with PGO multi-stage build
2. **scripts/pgo-profile.sh** - New script for workload execution
3. **x/src/main.rs:630-634** - Fix "Playwright" → "Puppeteer" typo

### Containerfile Structure

```dockerfile
# Stage 1: Base image with all tools
FROM rust:slim AS base
RUN apt-get update && apt-get install -y \
    llvm-14-tools \
    deno \
    chromium \
    xvfb \
    && rm -rf /var/lib/apt/lists/*
RUN rustup component add llvm-tools-preview

# Stage 2: Build instrumented binary
FROM base AS instrumented
COPY . /app
WORKDIR /app
ENV RUSTFLAGS='-Cprofile-generate=/app/profiling'
RUN mkdir -p /app/profiling
RUN cargo build --release

# Stage 3: Profile collection
FROM instrumented AS profiler
COPY scripts/pgo-profile.sh /scripts/
RUN /scripts/pgo-profile.sh
RUN /usr/lib/llvm-14/bin/llvm-profdata merge \
    -sparse /app/profiling/*.profraw \
    -o /app/profiling/merged.profdata

# Stage 4: Build optimized binary
FROM base AS optimizer
COPY --from=profiler /app/profiling/merged.profdata /app/profiling/
COPY . /app
WORKDIR /app
ENV RUSTFLAGS='-Cprofile-use=/app/profiling/merged.profdata'
RUN cargo build --release

# Stage 5: Minimal runtime
FROM scratch AS runtime
COPY --from=optimizer /app/target/release/quest-log /usr/local/bin/
# ... rest of runtime config
```

### pgo-profile.sh Script

```bash
#!/bin/bash
set -euo pipefail

# Start instrumented server in background
QUEST_LOG_DATA_DIR=/tmp/data PORT=3000 \
    ./target/release/quest-log &
SERVER_PID=$!

# Wait for server to be ready
for i in {1..30}; do
    if curl -s http://localhost:3000 > /dev/null 2>&1; then
        break
    fi
    sleep 1
done

# Run E2E tests (uses Puppeteer, not Playwright)
deno task test:e2e

# Graceful shutdown
kill -TERM $SERVER_PID
wait $SERVER_PID 2>/dev/null || true
```

## Rationale

1. **rust:slim over rust:alpine**: Debian-based image has better llvm-tools
   support; alpine requires musl-specific tooling that complicates PGO setup

2. **E2E tests as workload**: The existing Puppeteer E2E tests exercise all
   critical paths (database queries, template rendering, SSE), providing
   realistic profile data

3. **Automated in container build**: No manual profile management; builds are
   reproducible and CI-friendly

4. **Multi-stage keeps runtime small**: The final scratch image contains only
   the optimized binary, no profiling overhead

## Consequences

### Positive

- **Performance gain**: Expected 5-15% improvement based on PGO benchmarks for
  similar workloads
- **Automated**: No manual profile collection or management
- **Reproducible**: Same container build always produces same optimized binary
- **Real-world data**: E2E tests represent actual usage patterns
- **No runtime overhead**: Final binary has no instrumentation code

### Negative

- **Build time**: Increases from ~2min to ~6-8min (3-4x slower)
- **Complexity**: Containerfile becomes significantly more complex
- **Image size during build**: Builder stages are larger (Chrome, xvfb, llvm)
- **Debugging**: Build failures harder to diagnose with more stages

### Risks

- **Test coverage gaps**: If E2E tests don't cover hot paths, PGO benefit is
  reduced
  - Mitigation: Ensure E2E tests cover common operations
- **Profile staleness**: Code changes may make profiles less optimal over time
  - Mitigation: Profiles regenerated on every build
- **Build flakiness**: E2E tests in container may be flaky
  - Mitigation: Use same test setup as CI, xvfb for headless operation

## Implementation Notes

### Prerequisites

1. Ensure E2E tests pass locally: `deno task test:e2e`
2. Verify Chromium works in headless mode
3. Test llvm-tools installation on rust:slim

### Implementation Order

1. Create `scripts/pgo-profile.sh` with proper error handling
2. Create new Containerfile with PGO stages
3. Fix typo in `x/src/main.rs:630-634` ("Playwright" → "Puppeteer")
4. Update `documentation/adrs/README.md` index
5. Test with `cargo x container build`
6. Benchmark before/after with realistic workload

### Benchmarking

```bash
# Before PGO
cargo x container build
# Deploy and measure response times, throughput

# After PGO
# (with new Containerfile)
cargo x container build
# Deploy and measure same metrics
```

Key metrics to compare:

- Request latency (p50, p95, p99)
- Requests per second
- Memory usage
- Binary size

### Rollback Plan

Keep the old Containerfile as `Containerfile.nopgo` for quick rollback if PGO
builds prove problematic in production.

## Related Decisions

- ADR-001: Use SQLX_OFFLINE for Container Release Builds - Both optimize
  container builds
- ADR-008: Cargo xtask Pattern - Container build runs through
  `cargo x container`

## References

- [cargo-pgo documentation](https://github.com/Kobzol/cargo-pgo)
- [Rust PGO guide](https://doc.rust-lang.org/rustc/profile-guided-optimization.html)
- [LLVM PGO documentation](https://llvm.org/docs/HowToBuildWithPGO.html)
