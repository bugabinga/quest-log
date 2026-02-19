# Quest Log Agent Instructions

## Essential Directives

**ALWAYS** use the just MCP tools (`just_dev`, `just_test`, `just_verify`,
`just_lint`, `just_check`, `just_default`, `just_watch`) to run just commands.
Never use bash with `just` command - always use the dedicated just MCP tools.

All build/deploy/maintenance commands MUST run through the justfile. Direct
`cargo`, `rustc`, or other tool commands are forbidden. If a command is missing
from the justfile, add it there instead.

## Code Quality

Extreme high production quality, well compressed (DRY), simple structural style
avoiding abstractions and indirection.

## Testing

**Unit Tests:** `src/*` - fast feedback for core
logic

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

## File Structure

```
/src
  /main.rs /database.rs /models.rs
  /handlers/{mod,quests,parent,stats}.rs
  /templates/ /static/css/images/
/migrations/ /tests/
```

## Commit Guidelines

Present tense, focused single changes, reference issues.

## Security

Validate inputs, parameterized queries, sanitize HTML output.

## Documentation

rustdoc all public APIs, keep README updated.
