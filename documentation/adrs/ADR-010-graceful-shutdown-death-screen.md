# ADR-010: Graceful Shutdown Pattern (Death Screen)

## Status

Accepted

## Context

Quest Log is a web application that may need to restart during deployments or
maintenance. When the server restarts:

- Connected clients lose their SSE (Server-Sent Events) connection
- Users might see broken UI or frozen state
- Without handling, users might think the app is broken
- This is especially problematic for a kids' app where users may not understand
  technical issues

We needed a way to handle server restarts gracefully, providing clear feedback
to users and automatic recovery when the server returns.

## Decision Drivers

- **Must have**: Users must see clear feedback when server is unavailable
- **Must have**: Application must automatically recover when server returns
- **Should have**: Minimize user confusion, especially for young users
- **Should have**: No data loss during restart (state is server-side)
- **Could have**: Fun/themed presentation that fits the "quest" theme
- **Won't have**: Offline mode with local storage

## Considered Options

### Option A: Two-Phase Death Screen (Chosen)

Server broadcasts shutdown events before dying, client shows themed overlay with
health polling until server returns.

- **Pros**:
  - Clear visual feedback for users
  - Automatic recovery without manual refresh
  - Themed "realm rebirth" fits the game aesthetic
  - Health polling provides reliable server detection
- **Cons**:
  - Adds ~500ms to shutdown time
  - Additional client-side complexity
  - Proactive health checks consume resources

### Option B: Immediate Disconnect Handling

Let SSE onerror handle disconnection, show generic error, require manual
refresh.

- **Pros**:
  - Simpler implementation
  - No extra shutdown delay
- **Cons**:
  - Users see broken state until they manually refresh
  - No indication that server is coming back
  - Confusing for non-technical users (especially kids)

### Option C: No Special Handling

Rely on browser default behavior for connection loss.

- **Pros**:
  - Zero implementation effort
- **Cons**:
  - Users see frozen/broken UI
  - No recovery mechanism
  - Poor user experience
  - May appear as a bug

## Decision

We implemented a **two-phase graceful shutdown with death screen overlay**:

### Server-Side Behavior

When server receives shutdown signal (SIGINT/SIGTERM):

1. Broadcast `server-death` event to all connected SSE clients
2. Wait 500ms for clients to receive the message
3. Broadcast `shutdown-complete` event
4. Exit process

### Client-Side Behavior

The client handles server unavailability through multiple mechanisms:

1. **Proactive Health Check**: Polls `/health` every 5s to detect server death
   even if SSE doesn't fire onerror
2. **SSE Event Listeners**: Listens for `server-death` and `shutdown-complete`
   events
3. **Death Screen Overlay**: Shows themed "The Realm Rebirths" overlay with:
   - Skull icon and game-themed messaging
   - Animated progress bar
   - "Waiting for realm to revive..." status
   - Reassurance that progress is safe
4. **Health Polling**: Polls `/health` every 2s until server returns
5. **Automatic Reconnection**: When server returns, overlay hides and SSE
   reconnects automatically

### Implementation Details

**Server (`src/main.rs`)**:

- Handles SIGINT (Ctrl+C) and SIGTERM signals
- Uses tokio::select! to wait for shutdown signals
- Exits cleanly with `std::process::exit(0)`

**Client (`static/js/app.js`)**:

- `startProactiveHealthCheck()`: 5s interval health check
- `showShutdownOverlay()`: Creates themed overlay element
- `startServerPolling()`: 2s interval poll during shutdown
- `hideShutdownOverlay()`: Removes overlay on reconnection
- Event listeners for `server-death` and `shutdown-complete`

**Health Endpoint (`GET /health`)**:

- Returns "OK" with 200 status when server is healthy
- Used for both proactive checks and shutdown polling

## Rationale

This approach was chosen because:

1. **User Experience**: Kids using the app see a themed "realm rebirth" message
   instead of a broken interface, making server restarts feel like part of the
   game
2. **Automatic Recovery**: No manual intervention required - when the server
   comes back, the app reconnects automatically
3. **Reliability**: Multiple detection mechanisms (SSE events + health polling)
   ensure the death screen appears even if one mechanism fails
4. **Data Safety**: All state is server-side, so no data is lost during restart
5. **Thematic Consistency**: The "realm rebirth" theme fits the fantasy quest
   aesthetic of the application

The 500ms shutdown delay is acceptable because:

- Deployments are infrequent
- The delay ensures clients receive the shutdown message
- Better UX justifies the small operational cost

## Consequences

### Positive

- **Clear user feedback**: Users understand the app is temporarily unavailable
- **Automatic recovery**: No manual refresh required
- **Themed experience**: Fits the game aesthetic, less scary for kids
- **Reliable detection**: Multiple mechanisms ensure overlay appears
- **No data loss**: Server-side state is preserved across restarts

### Negative

- **Shutdown delay**: Adds ~500ms to server shutdown time
- **Client complexity**: Additional JavaScript for health polling and overlay
- **Resource usage**: Proactive health checks every 5s consume minimal resources

### Risks

- **Mitigated**: Network issues could trigger false death screens - health
  polling retries every 2s to handle transient failures
- **Mitigated**: SSE reconnection might fail - health polling provides backup
  detection mechanism

## Implementation Notes

### Adding New Client-Side Shutdown Behavior

1. Add event listener for `server-death` or `shutdown-complete` in the SSE
   section of `static/js/app.js`
2. Use `showShutdownOverlay()` / `hideShutdownOverlay()` functions
3. Test by killing the server during an active session

### Modifying Shutdown Timing

- Server-side timing is controlled by the broadcast delay in `src/main.rs`
- Client-side polling interval is in `startServerPolling()` (2s default)
- Proactive health check interval is in `startProactiveHealthCheck()` (5s
  default)

### Testing

Run integration tests:

```bash
cargo x verify
```

The `tests/graceful_shutdown.rs` file contains tests for shutdown behavior.

## Related Decisions

- ADR-003: Use Tracing for Structured Logging - provides observability for
  shutdown events

## References

- `ARCHITECTURE.md` - Death Screen (Graceful Shutdown) section
- `static/js/app.js` - Section 7: SSE Events - Connection & Death Screen
- `src/main.rs` - Shutdown signal handling
- `src/handlers/events.rs` - SSE endpoint implementation
