# ADR-011: Systemd Integration for Linux Deployments

## Status

Accepted

## Context

When deploying the application on Linux systems, particularly in production
environments, we need proper service management capabilities:

1. **Service Lifecycle**: The service manager needs to know when the application
   is ready to accept connections
2. **Health Monitoring**: Detecting hung or crashed processes requires watchdog
   support
3. **Socket Activation**: Allowing systemd to manage socket creation enables
   on-demand service activation and better socket coordination
4. **Linux Prevalence**: Systemd is the init system on most modern Linux
   distributions

## Decision Drivers

- **Must have**: Proper service startup notification for production reliability
- **Must have**: Optional feature to maintain cross-platform support
- **Should have**: Watchdog support for automatic crash recovery
- **Should have**: Socket activation for on-demand service and zero-downtime
  upgrades
- **Could have**: Pure Rust implementation without external C library
  dependencies
- **Won't have**: Support for non-systemd init systems (OpenRC, runit, etc.)

## Considered Options

### Option A: Full Systemd Integration

- **Pros**:
  - Native integration with Linux service management
  - Watchdog enables automatic restart on hang/crash
  - Socket activation allows on-demand startup
  - Zero-downtime upgrades possible with socket passing
- **Cons**:
  - Linux-only functionality
  - Adds complexity to deployment configuration

### Option B: Docker-Only Deployment

- **Pros**:
  - Platform-agnostic
  - Container orchestration handles health checks
- **Cons**:
  - Requires container runtime
  - No native socket activation
  - Health checks are HTTP-based, not process-based

### Option C: Simple Background Process

- **Pros**:
  - Simplest deployment model
  - Works everywhere
- **Cons**:
  - No service management integration
  - No automatic crash recovery
  - Manual port management

## Decision

We implement optional systemd integration as a feature flag (`systemd`) that is
Linux-only. The integration is implemented in `src/systemd.rs` and provides:

### 1. Ready Notification

Sends `READY=1` via `sd_notify` when the HTTP listener is ready:

```rust
pub fn notify_ready(status: Option<&str>) {
    if let Some(s) = status {
        let _unused = sd_notify::notify(&[NotifyState::Status(s)]);
    }
    let _unused = sd_notify::notify(&[NotifyState::Ready]);
}
```

### 2. Watchdog Heartbeat

If `WATCHDOG_USEC` is set (via `WatchdogSec=` in unit file), spawns a background
task that sends heartbeats at 1/3 of the configured interval:

```rust
pub fn start_watchdog() -> Option<JoinHandle<()>> {
    if let Some(duration) = sd_notify::watchdog_enabled() {
        let interval = Duration::from_micros(usec / 3);
        Some(tokio::spawn(async move {
            loop {
                sd_notify::notify(&[NotifyState::Watchdog]);
                tokio::time::sleep(interval).await;
            }
        }))
    } else {
        None
    }
}
```

### 3. Socket Activation

Detects and uses socket file descriptors passed by systemd (`LISTEN_FDS`):

```rust
pub fn take_listen_fds() -> Vec<RawFd> {
    // Reads LISTEN_FDS and LISTEN_PID environment variables
    // Returns fds starting at SD_LISTEN_FDS_START (3)
    // Clears env vars to prevent inheritance by children
}
```

### 4. Feature Gate

Dependency is conditionally included for Linux only:

```toml
[target.'cfg(target_os = "linux")'.dependencies]
sd-notify = "*"
```

Usage in code is gated with `#[cfg(target_os = "linux")]` and
`--features
systemd`.

## Rationale

- **sd-notify crate**: Pure Rust implementation, no `libsystemd` linking
  required. Reduces build complexity and container size.
- **1/3 watchdog interval**: Conservative heartbeat ensures notification arrives
  before timeout even under load.
- **Optional feature**: Non-Linux platforms and container deployments work
  without modification.
- **Socket activation**: Enables zero-downtime upgrades by having systemd manage
  the listening socket.

## Consequences

### Positive

- **Production reliability**: Systemd can detect and restart hung processes
- **Clean startup**: Service manager knows when application is ready
- **On-demand activation**: Service can start only when first connection arrives
- **Zero-downtime upgrades**: Socket activation enables seamless version
  upgrades
- **No external dependencies**: Pure Rust implementation, no libsystemd

### Negative

- **Linux-only**: Feature unavailable on macOS, Windows, BSD
- **Deployment complexity**: Requires systemd unit file configuration
- **Feature flag maintenance**: Additional conditional compilation paths

### Risks

- **Fd leak**: If socket fds aren't consumed correctly, could leak file
  descriptors. Mitigated by clearing `LISTEN_FDS` env var after reading.
- **Watchdog task leak**: Background task must be properly aborted on shutdown.

## Implementation Notes

### Unit File Example

**my-server.service**:

```ini
[Unit]
Description=Quest Log Server
Requires=my-server.socket

[Service]
Type=notify
WatchdogSec=30s
ExecStart=/usr/local/bin/quest-log
Restart=always

[Install]
WantedBy=multi-user.target
```

**my-server.socket**:

```ini
[Unit]
Description=Quest Log Socket

[Socket]
ListenStream=3000

[Install]
WantedBy=sockets.target
```

### Building with Systemd Support

```bash
cargo build --release --features systemd
```

### Code Integration Points

1. **main.rs**: Check for socket activation fds before binding
2. **After server start**: Call `notify_ready()` with optional status message
3. **Startup**: Call `start_watchdog()` and store the `JoinHandle` for cleanup

## Related Decisions

- ADR-003: Use Tracing for Structured Logging - systemd status messages
  complement structured logging

## References

- [sd_notify(3)](https://www.freedesktop.org/software/systemd/man/sd_notify.html)
- [sd_listen_fds(3)](https://www.freedesktop.org/software/systemd/man/sd_listen_fds.html)
- [sd-notify crate](https://docs.rs/sd-notify)
- [systemd.socket(5)](https://www.freedesktop.org/software/systemd/man/systemd.socket.html)
