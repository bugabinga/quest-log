# ADR-001: Use SQLX_OFFLINE for Container Release Builds

## Status

Proposed

## Context

The current container build process (via `Containerfile`) runs
`cargo build --release` without access to a database. This works because sqlx
connects to the database at runtime, but it means:

1. **Slow container builds**: Compile-time SQL verification is skipped anyway
   (no DB connection), wasting ~30s trying to establish one
2. **No compile-time SQL validation**: In release builds, schema errors only
   appear at runtime

The development workflow benefits from compile-time SQL checks (catches errors
early), but container builds cannot use them.

## Decision

Propose implementing SQLX_OFFLINE mode for container release builds:

1. **Generate query cache locally**: Run `cargo sqlx prepare` after migrations
   (requires local database)
2. **Commit `sqlx-data.json`**: Track the cache file in git
3. **Copy cache into container**: Add to `Containerfile` build stage
4. **Enable offline mode**: Set `ENV SQLX_OFFLINE=true` in runtime stage

### Containerfile changes:

```dockerfile
# Builder stage - copy cache file
COPY sqlx-data.json ./

# Runtime stage  
ENV SQLX_OFFLINE=true
```

### Build flow:

```
# Developer machine (has DB)
cargo sqlx prepare  # generates sqlx-data.json
git add sqlx-data.json

# CI / Container (no DB needed)
SQLX_OFFLINE=true cargo build --release
```

## Consequences

### Good

- Faster container builds (~30s improvement)
- No DB credentials needed in container build environment
- Consistent build behavior (same cache used everywhere)

### Bad

- Added complexity: must regenerate cache after schema changes
- Schema errors only caught at runtime in container builds
- Must remember to run `cargo sqlx prepare` after migrations
- Need process to update cache before release

### Ugly

- Cache file can become stale if not updated
- May need documentation for contributors

## Considered Alternatives

### Option A: Keep current setup (no offline mode)

- **Pros**: Simple, compile-time checks in dev catch errors early
- **Cons**: Slower container builds, no validation in release

### Option B: Use sqlx with runtime-only queries

- **Pros**: No compile-time overhead at all
- **Cons**: Loses all type safety, runtime errors instead of compile-time

### Option C: This proposal (SQLX_OFFLINE)

- **Pros**: Fast builds, maintains dev-time validation
- **Cons**: Added complexity, requires cache management

## Implementation Steps

1. Run `cargo sqlx prepare --lib` to generate `sqlx-data.json`
2. Add to `.gitignore` (or commit if small enough)
3. Update `Containerfile` to copy cache and set env var
4. Add CI step to verify cache is up-to-date
5. Document in CONTRIBUTING.md

## Open Questions

- How to handle cache validation in CI? (could fail if stale)
- Should cache be in git or regenerated in CI with temp DB?
- What's the acceptable latency between schema change and cache update?
