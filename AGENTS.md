# Quest Log Agent Instructions

## Essential Directives

**ALWAYS** use `cargo x` for development tasks. Commands like `cargo x test`,
`cargo x lint`, `cargo x check` should be used instead of calling cargo
directly.

All build/deploy/maintenance commands MUST run through `cargo x`. Direct
`cargo`, `rustc`, or other tool commands are forbidden. If a command is missing
from `x/src/main.rs`, add it there instead.

## Skills

Load these skills for this codebase:

- **rust** - Core Rust patterns and build optimization
- **rust-maud** - HTML template compilation
- **datastar** - Frontend reactivity and SSE
- **logging-monitoring** - Tracing and observability
- **testing-rust** - Unit and integration tests
- **error-handling** - Error patterns
- **security-checklist** - Input validation
- **git-commit** - Semantic commits, conventional format
- **git-merge** - Merge branches, resolve conflicts
- **git-branch** - Feature branch management

Use `/skill name <name>` to load before working on relevant tasks.

## Code Quality

Extreme high production quality, well compressed (DRY), simple structural style
avoiding abstractions and indirection.

## Testing

**Unit Tests:** `src/*` - fast feedback for core logic

**Integration Tests:** `tests/` directory - complete workflows

Use in-memory SQLite databases for isolation.

## Code Style

### Rust

- snake_case functions/variables, PascalCase structs/enums/traits
- SCREAMING_SNAKE_CASE constants
- `Result<T, E>` for errors, avoid `unwrap()`/`expect()`
- `async fn` with tokio, `Arc<Mutex<T>>` for shared state
- Group imports: std → external → local

### HTML/CSS

- Semantic HTML, BEM naming, flexbox/grid layouts, mobile-first
- Never hardcode colors - derive everything from 3 base colors
- Use data attributes for datastar

## Commit Guidelines

Present tense, focused single changes, reference issues.

## Security

Validate inputs, parameterized queries, sanitize HTML output.

## Documentation

rustdoc all public APIs. See ARCHITECTURE.md for architecture details,
logging/tracing setup, and ADRs in documentation/adrs/.
