# Editor Page Architecture

This document describes the architecture of the Editor Page feature for Quest Log.
The content below should be integrated into the main ARCHITECTURE.md file.

## Editor Page Section (to add to ARCHITECTURE.md)

Add this section after the "Routes" section in ARCHITECTURE.md:

```markdown
### Editor Page

The Editor Page provides a password-protected admin interface for managing
quests, rewards, and application settings.

#### Editor Routes

All routes under `/editor/*` are protected by session-based authentication:

| Route | Method | Description |
|-------|--------|-------------|
| `/editor` | GET | Editor page (shows login if unauthenticated) |
| `/editor/login` | POST | Authenticate with password |
| `/editor/logout` | POST | End session and show login |
| `/editor/quests` | GET | List all quests (JSON) |
| `/editor/quests` | POST | Create new quest (multipart) |
| `/editor/quests/{id}` | PUT | Update quest |
| `/editor/quests/{id}` | DELETE | Delete quest |
| `/editor/rewards` | GET | List all rewards (JSON) |
| `/editor/rewards` | POST | Create new reward (multipart) |
| `/editor/rewards/{id}` | PUT | Update reward |
| `/editor/rewards/{id}` | DELETE | Delete reward |
| `/editor/settings` | GET | Get current settings (JSON) |
| `/editor/settings` | PUT | Update settings |

#### Authentication System

**Password Hashing:**

- Uses Argon2id (via `argon2` crate) for secure password hashing
- Passwords are never stored in plaintext
- Hash verification uses constant-time comparison to prevent timing attacks

**Session Management:**

- Sessions are stored in-memory (`Arc<RwLock<HashMap<String, Instant>>>`)
- Each session has a unique cryptographically-secure token
- Default session duration: 24 hours
- Sessions are validated on each request by checking expiry time
- Session tokens are passed via `x-editor-session` header

**Rate Limiting:**

- Maximum 5 login attempts per minute per IP address
- Uses sliding window algorithm (60-second window)
- Tracked in-memory with automatic cleanup of expired attempts
- Failed attempts are recorded; successful logins clear the rate limit
- Rate-limited requests return a user-friendly error message

#### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `QUEST_LOG_EDITOR_PASSWORD_HASH` | Argon2 hash of the editor password | (required) |
| `QUEST_LOG_EDITOR_SESSION_DURATION_HOURS` | Session lifetime in hours | 24 |

#### CLI Commands

```bash
# Generate password hash for environment variable
cargo x run -- editor-password
```

This command interactively prompts for a password, confirms it, and outputs
the Argon2 hash to add to your `.env` file or environment.

#### Security Considerations

1. **Password Storage**: Never store the password itself; only the Argon2 hash
   in `QUEST_LOG_EDITOR_PASSWORD_HASH`

2. **Rate Limiting**: Prevents brute-force attacks by limiting login attempts
   per IP address. After 5 failed attempts within a minute, further attempts
   are blocked until the window expires.

3. **Session Tokens**: Generated using cryptographically secure random bytes
   (via `rand::rngs::OsRng`). Tokens are long and unique to prevent guessing.

4. **Session Expiry**: Sessions automatically expire after 24 hours (or
   configured duration), requiring re-authentication.

5. **Timing Attack Prevention**: Argon2's `verify_password` uses constant-time
   comparison, preventing attackers from determining password length or
   partial matches through timing analysis.

6. **IP-Based Rate Limiting**: Uses `x-forwarded-for` or `x-real-ip` headers
   to identify clients (for reverse proxy compatibility), falling back to
   "unknown" if neither header is present.

#### Editor UI (`src/ui/editor.rs`)

Server-side HTML rendering using Maud templates:

- `editor_page` - Full page with tabs for Quests, Rewards, Settings
- `editor_quests_panel` - Quest management table and form
- `editor_rewards_panel` - Reward management table and form
- `editor_settings_panel` - Weekly EXP goal configuration
- `auth_modal` - Login form (in `src/ui/auth.rs`)

#### Editor JavaScript (`static/js/editor.js`)

Client-side interactions:

- Tab switching between Quests/Rewards/Settings
- Form visibility toggles
- Image upload preview with validation (max 5MB, image types only)
- Toast notifications for success/error feedback
- Form submission handlers using Datastar SSE

#### Editor Handlers (`src/handlers/editor/mod.rs`)

HTTP handlers for all editor operations:

- `editor_page_handler` - Renders editor or login based on auth status
- `login_handler` - Processes login with rate limiting and session creation
- `logout_handler` - Ends session
- `create_quest_handler` / `create_reward_handler` - Handle multipart forms
  with optional image uploads
- CRUD operations for quests, rewards, and settings
```

## Additional Updates to ARCHITECTURE.md

### Update `src/state.rs` description:

```markdown
### `src/state.rs`

Application state: Database + broadcast channel + editor sessions + rate limiter.
```

### Update `src/auth.rs` section (add new section):

```markdown
### `src/auth.rs`

Authentication module providing:

- `hash_password` - Argon2id password hashing
- `verify_password` - Constant-time password verification
- `generate_session_token` - Cryptographically secure token generation
- `LoginRateLimiter` - IP-based rate limiting for login attempts
- `get_password_hash_from_env` - Read password hash from environment
```

### Update `src/ui/` section (add editor entries):

```markdown
### `src/ui/` (Maud templates)

Server-side HTML rendering using Maud:

- `base.rs` - Base template with common HTML scaffolding (head, nav, video
  modal)
- `quests.rs` - Main page template (uses base.rs)
- `bounty.rs` - Bounty Board page for weekly rewards
- `error.rs` - Error page template (uses base.rs)
- `editor.rs` - Editor page with tabs for Quests, Rewards, Settings
- `auth.rs` - Login modal for editor authentication
- `fragments/` - Reusable UI components (toggle, nav_buttons, etc.)
  - `weekly_rewards.rs` - Weekly rewards display with celebration animations
```

### Update `static/` section (add editor.js):

```markdown
### `static/`

Static assets (generated from `assets/` via `cargo x assets`):

- `js/app.js` - Consolidated application JavaScript (SSE, UI interactions, death
  screen)
- `js/editor.js` - Editor-specific JavaScript (tab switching, form handling)
- `js/datastar.js` - Datastar library for reactive UI
- `style.css` - All styles including death screen overlay
- `images/`, `video/`, `fonts/`
- `manifest.json` - PWA manifest
- `favicon.png` - 32x32 favicon
```

### Update Environment Variables section:

```markdown
## Environment Variables

- `QUEST_LOG_DB` - Path to SQLite database (default: `~/.quest-log/questlog.db`)
- `QUEST_LOG_PORT` - HTTP port (default: 3000)
- `QUEST_LOG_TODAY` - Override "today" for testing (format: YYYY-MM-DD)
- `QUEST_LOG_EDITOR_PASSWORD_HASH` - Argon2 hash for editor authentication
- `QUEST_LOG_EDITOR_SESSION_DURATION_HOURS` - Session lifetime (default: 24)
- `RUST_LOG` - Log level filter (default:
  `quest_log=debug,tokio=info,axum=warn`)
```

### Add to JavaScript Structure section:

```markdown
### `static/js/editor.js`

Editor-specific JavaScript for the admin interface:

- Tab switching between Quests, Rewards, Settings panels
- Form visibility toggles for add/edit operations
- Image upload preview with file validation
- Toast notifications for user feedback
- Form submission handlers using Datastar SSE
```

## File Locations

| Component | File Path |
|-----------|-----------|
| Auth module | `src/auth.rs` |
| Editor handlers | `src/handlers/editor/mod.rs` |
| Editor UI templates | `src/ui/editor.rs` |
| Auth UI (login modal) | `src/ui/auth.rs` |
| Editor JavaScript | `static/js/editor.js` |
| CLI command | `src/cli.rs` |
| App state (sessions) | `src/state.rs` |
