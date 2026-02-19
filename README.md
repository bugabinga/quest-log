# The Quest Log

This application is a gameified TODO-list for children. It is meant for kids to
track their daily tasks in a fun way.

TODO-items are set by parents. Items can be checked of for each day, in order to
earn EXP (experience points). Parents can set a weekly goal of EXP.

## Architecture

The app is a very simple web app. No Logins. Only single user mode. The /
endpoint points to the main UI, the quest log. The /parent endpoint points to a
settings page (for parents) to configure EXP goals and set tasks per weekday.
They can also set a weekly reward for the child.

State is managed by the server in a sqlite database. The web app is a simple
Hypermedia server side rendered site, with interactivity implementen in HTML
with datastar. The backend is a simple rust HTTP server (no TLS).

All build, deploy and maintenance commands are centralized in the justfile.

## Installation & Setup

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- SQLite3 (usually pre-installed on most systems)

### Quick Start

```bash
# Clone the repository
git clone <repository-url>
cd quest-log

# Install dependencies
cargo build

# Set up database
just db-migrate

# Start development server
just dev
```

Open http://localhost:3000 in your browser.

## Development

### Environment Variables

The application uses the following environment variables:

- `QUEST_LOG_DATA_DIR` - Directory path for storing the SQLite database file
  (must be absolute path, default: current directory)
- `PORT` - Server port (default: `3000`)

### Development Commands

```bash
# Start development server with hot reload
just dev

# Run tests
just test

# Run linter and formatter
just lint

# Reset database
just db-reset
```

## Architecture Details

**Backend Stack:**

- **Language:** Rust
- **Web Framework:** axum (async HTTP server)
- **Database:** SQLite with sqlx (compile-time checked queries)
- **HTML Templating:** askama (compile-time template validation)
- **Frontend Interactivity:** datastar (server-driven reactive UI)
- **Error Handling:** thiserror with HTTP status code mapping

**Key Design Patterns:**

- Server-side state management with datastar SSE streams
- Compile-time safety for SQL queries and HTML templates
- Simple module structure with clear separation of concerns
- Environment-based configuration

**Project Structure:**

```
src/
├── main.rs           # Server entry point
├── lib.rs            # Core business logic
├── database.rs       # SQLite operations
├── models.rs         # Data structures
├── handlers/         # HTTP route handlers
│   ├── quests.rs     # Main UI endpoints
│   ├── parent.rs     # Settings endpoints
│   └── stats.rs      # Highscore endpoints
└── templates/        # HTML templates
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make changes following the guidelines in `AGENTS.md`
4. Run tests: `just test`
5. Run linting: `just lint`
6. Commit with clear messages
7. Submit a pull request

## The main UI: quest log

Endpoint: /

The quest log is the main interaction point of the app for kids. They can track
their progress and see their tasks. They can also see their current and missing
EXP and expected weekly reward. By default it shows the quest log for the
current day, but navigation to other days is possible. Quest items can be
checked or unchecked, increasing or decreasing the total weekly EXP respectivly.

## The parent UI: settings

Endpoint: /parent

The settings page allows to set up quest to do for children per day of the week.
Each quest consists of:

- a title
- a description
- an image
- EXP to gain from achieving this quest

Parents may also set weekly EXP goals, the amount of EXP the child must reach
per week to gain a reward. Rewards can be defined in the UI. A reward consists
of:

- a title
- an image

By default, the all images are AI-generated (by some AI API freely available),
but custom images may be set.

There is also a button to reset the history (danger!).

## The stats UI: highscore

Endpoint: /highscore

The highscore screen is an aggregation of historical data over achived quests
and received rewards. It also displays the total EXP.
