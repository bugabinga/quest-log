# ADR-007: Server-Driven UI with Datastar

## Status

Accepted

## Context

Quest Log needs an interactive, real-time UI where users can complete quests,
claim rewards, and see updates immediately. Traditional approaches require
either:

1. Full page reloads (poor UX)
2. Client-side state management (complexity, sync issues)
3. API layer + SPA framework (duplication, cognitive overhead)

The core insight is that Quest Log is fundamentally a server-centric
application:

- All data lives in SQLite on the server
- Business logic (XP calculation, quest completion) runs server-side
- Multiple clients need to see synchronized state in real-time
- The UI is relatively simple (lists, buttons, forms)

We needed an architecture that avoids client-side state synchronization while
still providing a reactive, app-like experience.

## Decision Drivers

### Must Have

- **Server as source of truth**: No client-side state duplication
- **Real-time updates**: All connected clients see changes immediately
- **No API layer**: Avoid maintaining separate REST/GraphQL endpoints
- **Progressive enhancement**: Works with basic HTML, enhanced by JavaScript

### Should Have

- **Simplicity**: Single mental model for rendering (server-side)
- **Type safety**: Compile-time verification of HTML templates
- **Small bundle size**: Fast initial load, minimal JavaScript

### Could Have

- **Offline support**: Currently not a priority
- **Complex client-side interactions**: Limited need for this

### Won't Have

- **Client-side routing**: All navigation through server
- **Client-side business logic**: Logic stays on server
- **Heavy JavaScript framework**: Avoid React/Vue complexity

## Considered Options

### Option A: Datastar with Server-Sent Events (SSE)

**Architecture**: Server renders HTML fragments via Maud templates, sends them
over SSE to connected clients. Datastar handles DOM morphing and signal
management on the client.

- **Pros**:
  - Server is the single source of truth
  - No client-side state synchronization issues
  - HTML-over-the-wire (no separate API)
  - Real-time updates built-in via SSE broadcast
  - Small client library (~50KB)
  - Type-safe templates with Maud
- **Cons**:
  - Requires persistent SSE connection
  - More complex than simple page reloads
  - Less ecosystem support than HTMX

### Option B: HTMX

**Architecture**: Similar to Datastar, but uses HTML attributes for
interactions. Server returns HTML fragments via HTTP requests.

- **Pros**:
  - Larger community and ecosystem
  - Well-documented patterns
  - No persistent connection required
- **Cons**:
  - Real-time updates require polling or separate WebSocket setup
  - Less built-in signal management
  - Verbose attribute syntax

### Option C: React/Vue SPA

**Architecture**: Client-side SPA with API backend. React/Vue manages state,
calls REST/GraphQL endpoints for data.

- **Pros**:
  - Rich ecosystem, extensive tooling
  - Complex client-side interactions possible
  - Offline support easier to implement
- **Cons**:
  - Duplicate state (client + server)
  - Requires maintaining API layer
  - Larger bundle size
  - State synchronization complexity
  - Overkill for this use case

### Option D: Vanilla JavaScript

**Architecture**: Hand-written JavaScript making fetch requests, manually
updating DOM.

- **Pros**:
  - No framework dependency
  - Full control
- **Cons**:
  - Significant boilerplate
  - Easy to introduce bugs
  - No structure or patterns
  - Difficult to maintain as complexity grows

## Decision

We chose **Datastar with SSE** for server-driven UI:

1. **Datastar** handles reactive DOM updates and signal management
2. **SSE** provides real-time push from server to all clients
3. **Maud** templates generate HTML server-side with compile-time verification
4. **Broadcast channel** in Rust pushes updates to all connected SSE streams

### How It Works

```
┌─────────────┐     SSE      ┌─────────────┐
│   Client A  │◄────────────│             │
└─────────────┘              │             │
                             │   Server    │
┌─────────────┐     SSE      │             │
│   Client B  │◄────────────│             │
└─────────────┘              └──────┬──────┘
                                    │
                             ┌──────▼──────┐
                             │   SQLite    │
                             └─────────────┘
```

1. User action triggers HTTP POST (e.g., toggle quest)
2. Server updates database
3. Server broadcasts HTML fragment via channel
4. All connected SSE streams receive fragment
5. Datastar morphs DOM with new HTML
6. All clients see updated state instantly

### Key Components

**Server-side**:

- `src/handlers/` - HTTP handlers return HTML fragments or SSE streams
- `src/ui/fragments/` - Maud templates for reusable HTML components
- `src/state.rs` - Broadcast channel for SSE distribution

**Client-side**:

- `static/js/datastar.js` - Datastar library (~50KB)
- `static/js/app.js` - Minimal JS for SSE connection, death screen, UI effects

### Code Example

Server handler returns HTML fragment:

```rust
// src/handlers/quests.rs
pub async fn toggle_quest(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ToggleRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Update database
    state.db.toggle_quest_completion(payload.quest_id, payload.date).await?;
    
    // Broadcast updated quest HTML to all clients
    let fragment = quest_item_html(&quest, &completion_status);
    state.bcast.send(BroadcastMessage::PatchElements {
        html: fragment.into_string(),
        origin: payload.client_id,
    });
    
    Ok(Json(json!({ "exp_today": new_exp })))
}
```

Client receives and applies:

```javascript
// static/js/app.js
eventSource.addEventListener("datastar-patch-elements", (e) => {
  const payload = JSON.parse(e.data);
  applyPatchElements(payload.data); // Datastar morphs DOM
});
```

## Rationale

### Why Datastar over HTMX

Datastar provides built-in real-time support via SSE, while HTMX would require
additional setup for push updates. Datastar's signal system also provides a
clean way to pass structured data (like XP totals) alongside HTML fragments.

### Why not SPA

Quest Log doesn't need client-side state. The server already has all the data.
An SPA would duplicate state and require maintaining an API layer. The
hypermedia approach keeps logic in one place.

### Why Maud

Maud provides compile-time HTML validation. Template errors are caught at build
time, not runtime. This is especially valuable when HTML fragments are broadcast
to all clients—any error would affect everyone.

## Consequences

### Positive

- **No client-side state sync**: Server is always the source of truth
- **Simpler mental model**: One rendering paradigm (server-side HTML)
- **Real-time by default**: SSE broadcast ensures all clients stay synchronized
- **Type-safe templates**: Maud catches HTML errors at compile time
- **Small JavaScript footprint**: ~50KB for Datastar, minimal app code
- **No API maintenance**: No separate REST/GraphQL endpoints to maintain

### Negative

- **Requires SSE connection**: Client must maintain persistent connection
- **Connection management**: Death screen pattern needed for graceful handling
- **Less ecosystem**: Datastar is newer than HTMX, smaller community
- **Server resource usage**: Each client holds open an SSE connection

### Risks

| Risk                     | Mitigation                                                       |
| ------------------------ | ---------------------------------------------------------------- |
| SSE connection drops     | Health polling + death screen with auto-reconnect                |
| Server restart           | Two-phase shutdown (broadcast death, wait, close)                |
| Scaling SSE connections  | SQLite limits scale; could migrate to PostgreSQL + Redis pub/sub |
| Datastar library changes | Vendored in `static/js/`, version locked                         |

## Implementation Notes

### Adding New Interactive Features

1. Create Maud template in `src/ui/fragments/`
2. Add handler in `src/handlers/` that returns HTML fragment
3. Broadcast via `state.bcast.send(BroadcastMessage::PatchElements { ... })`
4. Client automatically receives and applies via SSE listener

### Death Screen Pattern

When server shuts down:

1. Broadcast `server-death` event to all SSE clients
2. Wait 500ms for clients to receive message
3. Broadcast `shutdown-complete`
4. Client shows death screen overlay
5. Client polls `/health` every 2s
6. On success, hide overlay, reconnect SSE

This ensures users see friendly message instead of broken UI during deployments.

### Datastar Signal Patches

For data that doesn't need HTML (e.g., XP counter), use signal patches:

```rust
state.bcast.send(BroadcastMessage::PatchSignals {
    signals: json!({ "exp_today": 150 }),
    origin: client_id,
});
```

Client handles via `datastar-signal-patch` event.

### Computed Signals

Datastar computed signals allow deriving frontend state from existing signals.
**Important**: Computed objects must be wrapped in parentheses to avoid
JavaScript interpreting the object literal `{}` as a block statement.

**Correct**:

```rust
let computed = r#"({expTodayPercent: () => Math.round($expToday / Math.max($expTodayMax, 1) * 100)})"#.to_string();
```

**Incorrect** (causes syntax errors):

```rust
let computed = r#"{expTodayPercent: () => Math.round($expToday / Math.max($expTodayMax, 1) * 100)}"#.to_string();
```

This pattern applies to all computed signal objects in `data-computed`
attributes.

## Related Decisions

- **ADR-002**: Vendoring Datastar in `static/` directory
- **ADR-003**: Tracing for observability (debugging SSE issues)

## References

- [Datastar Documentation](https://data-star.dev/)
- [Maud Documentation](https://maud.lambda.xyz/)
- [Server-Sent Events MDN](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events)
- [HTMX (alternative considered)](https://htmx.org/)
- [Hypermedia Systems (book)](https://hypermedia.systems/)
