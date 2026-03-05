# ADR-003: Use Tracing for Structured Logging and Observability

## Status

Accepted

## Context

We needed better observability for debugging production issues and understanding
request flow. The application had minimal logging - mostly ad-hoc
`tracing::debug!` and `tracing::error!` calls without:

- Automatic timing of operations
- Causal relationships between operations
- Queryable structured fields
- Error context propagation

## Decision

We adopted the `tracing` crate with the following approach:

### 1. Core Dependencies

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-error = "0.2"
```

- **tracing**: Core framework for instrumentation
- **tracing-subscriber**: Formatting/output layers
- **tracing-error**: Error propagation with span context

### 2. Error Layer Setup

Added `ErrorLayer` to capture span context with errors:

```rust
use tracing_error::ErrorLayer;

tracing_subscriber::registry()
    .with(filter)
    .with(fmt_layer)
    .with(ErrorLayer::default())
    .init();
```

This enables `SpanTrace` capture - when errors occur, the current span context
is attached for debugging.

### 3. Function Instrumentation

Added `#[instrument]` attribute to handlers and key database methods with custom
emoji names:

```rust
#[instrument(name = "📜 GET /", skip(state, query))]
pub async fn quests(...) 

#[instrument(name = "✨ toggle_quest", skip(state, request))]
pub async fn toggle_quest(...)

#[instrument(name = "📋 get_quests_for_day", skip(self))]
pub async fn get_quests_for_day(...)
```

### 4. Log Levels

Standard Rust log levels via `RUST_LOG` env var:

- `trace` - Most verbose, shows span enter/exit
- `debug` - Default for development
- `info` - General operational events
- `warn` / `error` - Warnings and errors

## Consequences

### Good

- **Automatic timing**: Every instrumented function shows duration
- **Causal chains**: Parent-child relationships between handler → DB spans
- **Structured fields**: Queryable attributes (quest_id, date, etc.)
- **Error context**: `tracing-error` captures span context with errors
- **No external dependencies**: Uses standard `tracing` crate, no OTLP export

### Bad

- **Compile-time macro overhead**: `#[instrument]` adds compile time
- **Verbosity at trace level**: Can be overwhelming
- **No distributed tracing**: Works within single process only

### Future Considerations

- Could add OTLP exporter for distributed tracing (Jaeger/Zipkin)
- Could add sampling for production trace volume control

## Alternative Considered

### OpenTelemetry

- Rejected: Adds significant complexity, external dependencies
- Would require OTLP collector/endpoint
- Overkill for single-service application

### Simple Logging Only

- Rejected: Loses automatic timing and causal relationships
- Hard to query/filter structured data

## Guidelines

1. **Instrument at boundaries**: HTTP handlers, DB operations
2. **Use emoji names**: Makes span names readable in logs
3. **Skip large params**: Don't capture entire structs in spans
4. **Include key fields**: quest_id, date, reward_id for debugging
5. **Use trace level sparingly**: For span timing, not general logging
