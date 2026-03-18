# ADR-006: Server-Sent Events for Real-Time Updates

## Status

Accepted

## Context

The Quest Log application needs real-time updates when users toggle quest
completion status. When one client marks a quest as complete, all other
connected clients should immediately see the updated state without manual page
refresh.

The use case is inherently **unidirectional**: the server pushes updates to
clients. There is no need for clients to send messages back through the same
channel (form submissions use regular HTTP POST).

## Decision Drivers

- **Must have**: Real-time updates when quests are toggled
- **Must have**: Works over standard HTTP (no special protocols)
- **Must have**: Automatic reconnection on connection drop
- **Should have**: Simple implementation and mental model
- **Should have**: Good browser support without polyfills
- **Could have**: Low bandwidth overhead
- **Won't have**: Bidirectional communication (not needed)

## Considered Options

### Option A: Server-Sent Events (SSE)

- **Pros**:
  - Native browser support via `EventSource` API
  - Unidirectional model matches our use case perfectly
  - Automatic reconnection built into the spec
  - Works over standard HTTP/1.1 and HTTP/2
  - Simpler than WebSockets (no handshake protocol)
- **Cons**:
  - One-way communication only (server to client)
  - Some proxy/buffering issues with older infrastructure

### Option B: WebSockets

- **Pros**:
  - Full bidirectional communication
  - Lower latency for high-frequency updates
  - Binary frame support
- **Cons**:
  - Overkill for unidirectional push
  - More complex protocol (handshake, frame encoding)
  - Requires separate endpoint handling
  - More infrastructure complexity (proxies, load balancers)

### Option C: HTTP Polling

- **Pros**:
  - Simplest to implement
  - Works everywhere
- **Cons**:
  - High latency (depends on poll interval)
  - Wastes bandwidth with empty responses
  - Server load scales with poll frequency

### Option D: HTTP Long Polling

- **Pros**:
  - Lower latency than regular polling
  - Works through most firewalls/proxies
- **Cons**:
  - Connection overhead per update
  - More complex than SSE
  - Not truly real-time

## Decision

We use **Server-Sent Events (SSE)** with a `tokio::sync::broadcast` channel for
real-time updates.

### Implementation Details

```rust
// Broadcast channel in AppState (capacity: 128)
pub bcast: broadcast::Sender<ServerMessage>

// SSE endpoint at GET /events
pub async fn events(State(state): State<AppState>) -> Sse<...> {
    let rx = state.bcast.subscribe();
    // Stream ServerMessage as SSE events
}

// Server message types
pub enum ServerMessage {
    Elements(String, Option<String>),  // HTML fragments
    Signals(String, Option<String>),   // JSON state updates
}
```

### Event Types

- `datastar-patch-elements`: HTML fragments for DOM morphing
- `datastar-patch-signals`: JSON state updates for client-side signals

### Client Integration

Datastar library consumes SSE events and performs DOM patching automatically. No
custom client-side SSE handling required.

## Rationale

1. **Unidirectional fit**: SSE is designed for server-to-client push, which is
   exactly what we need. No need for the complexity of bidirectional protocols.

2. **Simplicity**: SSE uses standard HTTP with a specific content type
   (`text/event-stream`). No special protocol or handshake required.

3. **Automatic reconnection**: The browser's `EventSource` API handles
   reconnection automatically when connections drop.

4. **Broadcast pattern**: `tokio::sync::broadcast` is ideal for fan-out to
   multiple subscribers. When one client toggles a quest, the server broadcasts
   to all connected clients.

5. **Datastar integration**: Datastar expects SSE for its reactive model, making
   this a natural fit.

## Consequences

### Positive

- **Simple architecture**: One-way data flow is easy to reason about
- **Native browser support**: No polyfills needed for modern browsers
- **HTTP/2 multiplexing**: Works alongside other requests on same connection
- **Automatic reconnect**: Browser handles reconnection on connection loss
- **Graceful degradation**: If SSE fails, page refresh still works
- **Integration with Datastar**: Library handles SSE parsing and DOM updates

### Negative

- **One-way only**: Cannot push from client to server (not needed here)
- **Connection limits**: Browsers limit concurrent SSE connections per origin (6
  for HTTP/1.1, unlimited for HTTP/2)
- **Proxy buffering**: Some proxies buffer SSE responses; requires
  `X-Accel-Buffering: no` header if needed

### Risks

- **Broadcast capacity**: If subscribers can't keep up, messages are dropped.
  Mitigated by 128 capacity and lightweight message handling.
- **Connection storms**: Many clients reconnecting simultaneously after
  deployment. Mitigated by browser's exponential backoff.

## Implementation Notes

1. **Broadcast channel capacity**: Set to 128 to handle bursts. Slow clients may
   miss messages but will catch up on next broadcast.

2. **Message origin tracking**: `ServerMessage` includes optional origin field
   to prevent echo loops (client ignoring its own updates).

3. **Graceful shutdown**: Server broadcasts `server-death` event before
   shutdown, allowing clients to show appropriate UI during reconnection.

4. **Event format**: Datastar expects specific event names (`datastar-patch-*`)
   and JSON payloads with `data` and `origin` fields.

## Related Decisions

- ADR-003: Tracing for observability of SSE connections
- ADR-002: Vendored Datastar library for client-side reactive updates

## References

- [MDN: Server-sent events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events)
- [tokio::sync::broadcast documentation](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html)
- [Datastar documentation](https://data-star.dev/)
