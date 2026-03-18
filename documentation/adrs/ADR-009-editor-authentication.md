# ADR-009: Editor Authentication Strategy

## Status

Accepted

## Context

Quest Log provides an editor interface (`/editor`) for the parent to manage
quests, bounties, and settings. This is an administrative interface that
requires authentication, but the application is designed for single-user
(family) use where only the parent needs access.

The editor needs to be protected from unauthorized access, but we don't need a
full user management system with multiple accounts, roles, or persistent
sessions across server restarts.

## Decision Drivers

- **Must have**: Secure authentication to prevent unauthorized access
- **Must have**: Protection against brute-force attacks
- **Should have**: Simple implementation without database dependencies for
  sessions
- **Should have**: Configurable session duration
- **Could have**: Persistent sessions across restarts
- **Won't have**: Multi-user support or role-based access control

## Considered Options

### Option A: In-Memory Sessions (Chosen)

Store sessions in an in-memory `HashMap<String, Instant>` with Argon2id password
hashing.

- **Pros**:
  - Simple implementation, no database schema changes
  - Fast session validation (no disk I/O)
  - Sessions automatically invalidated on server restart (security by default)
  - No external dependencies for session storage
- **Cons**:
  - Sessions lost on server restart (users must re-login)
  - Not suitable for horizontal scaling (but not needed for single-server app)

### Option B: JWT (JSON Web Tokens)

Stateless authentication with signed tokens containing expiry claims.

- **Pros**:
  - Stateless - no server-side storage needed
  - Survives server restarts
  - Can scale horizontally
- **Cons**:
  - Token revocation is complex (need blacklist or short expiry + refresh)
  - Larger payload size in each request
  - More complex to implement securely
  - Overkill for single-user application

### Option C: Database Sessions

Store session tokens in SQLite database alongside application data.

- **Pros**:
  - Persistent across restarts
  - Easy to query and manage sessions
  - Can implement session listing/revocation UI
- **Cons**:
  - Additional database I/O for each authenticated request
  - Need to clean up expired sessions periodically
  - Adds complexity to database schema
  - Unnecessary for single-user scenario

### Option D: OAuth / Third-Party Auth

Use external provider (Google, GitHub, etc.) for authentication.

- **Pros**:
  - No password management required
  - Familiar login flow for users
  - Leverages provider's security
- **Cons**:
  - External dependency and complexity
  - Requires internet connectivity
  - Overkill for self-hosted family application
  - Privacy concerns for some users

## Decision

We will use **in-memory sessions** with:

1. **Password Hashing**: Argon2id (via `argon2` crate)
   - Winner of the Password Hashing Competition (2015)
   - Memory-hard algorithm resistant to GPU attacks
   - Constant-time verification prevents timing attacks

2. **Session Tokens**: Cryptographically secure random 32-byte tokens
   - Generated using `OsRng` (operating system's CSPRNG)
   - Combined with random salt for additional entropy

3. **Session Storage**: In-memory `HashMap<String, Instant>`
   - Token mapped to expiry timestamp
   - Lazy cleanup on session validation
   - Default 24-hour expiry, configurable via
     `QUEST_LOG_EDITOR_SESSION_DURATION_HOURS`

4. **Rate Limiting**: IP-based protection
   - Maximum 5 login attempts per minute per IP address
   - Tracked in-memory with sliding window
   - Prevents brute-force attacks

5. **Cookie-Based Sessions**: HTTP-only cookies for session storage
   - Secure flag in production
   - Prevents JavaScript access (XSS protection)

## Rationale

For a single-user family application, in-memory sessions provide the best
balance of simplicity and security:

- **Simplicity**: No database migrations, no external services, minimal code
- **Security**: Industry-standard Argon2id hashing, rate limiting, secure tokens
- **Appropriate trade-off**: Losing sessions on restart is acceptable when only
  one user needs to re-login occasionally

The Argon2id algorithm was chosen because it provides the best protection
against both GPU-based and side-channel attacks while being the current
recommended standard for password hashing.

## Consequences

### Positive

- Simple, maintainable authentication code
- Fast session validation (no disk I/O)
- Industry-standard password security
- Built-in brute-force protection
- No database schema changes required
- Sessions automatically cleaned up on server restart

### Negative

- Users must re-login after server restart
- Cannot support multiple server instances without shared session store
- Session state is lost during deployments

### Risks

- **Memory exhaustion**: Malicious actors could attempt to create many sessions
  - _Mitigation_: Rate limiting prevents rapid session creation
  - _Mitigation_: Lazy cleanup removes expired sessions on validation
- **Session hijacking**: If cookie is stolen, attacker gains access
  - _Mitigation_: HTTP-only cookies prevent JavaScript access
  - _Mitigation_: Reasonable session expiry (24h default)

## Implementation Notes

### Configuration

```bash
# Required in production
export QUEST_LOG_EDITOR_PASSWORD_HASH="<argon2id-hash>"

# Optional - session duration in hours (default: 24)
export QUEST_LOG_EDITOR_SESSION_DURATION_HOURS=24
```

### Generating Password Hash

Use the application to generate a hash:

```rust
use quest_log::auth::hash_password;
let hash = hash_password("your-secure-password").unwrap();
println!("{}", hash);
```

### Development Mode

In debug builds (`cargo x run`), a default password "dev" is used for
convenience. **This must never be used in production.**

### Code Locations

- Password hashing and verification: `src/auth.rs`
- Session management: `src/state.rs`
- Configuration: `src/config.rs`
- Login/logout handlers: `src/handlers/editor.rs`

## Related Decisions

- ADR-001: Use SQLX_OFFLINE for Container Release Builds - authentication config
  is embedded at build time

## References

- [Argon2 RFC 9106](https://www.rfc-editor.org/rfc/rfc9106)
- [OWASP Session Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)
- [Password Hashing Competition](https://www.password-hashing.net/)
