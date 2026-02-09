# Agent Instructions for Quest Log

This file contains instructions for AI agents working on the Quest Log codebase.
Follow these guidelines strictly.

## Code Quality

- extreme high production quality
- well compressed (DRY) and optimized
- hand made style
- simple structural style, avoiding abstractions and indirection

## Tech Stack

This is a whitelist, no other tech is allowed

- git
- just
- HTML
- CSS
- datastar
- Rust
- Markdown
- sqlite

### Specific Architecture Choices

**Web Framework:** axum (simple, async, excellent datastar integration)
**Database:** sqlx with SQLite (async queries with compile-time checking) **HTML
Templating:** askama (compile-time checked templates) **Error Handling:**
thiserror (well-maintained with HTTP status code mapping) **Configuration:**
Environment variables only **Testing:** Unit tests for business logic +
integration tests for HTTP endpoints

### Dependencies

```
[dependencies]
axum = "0.8"
datastar = { version = "0.3", features = ["axum"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
askama = "0.12"
thiserror = "2.0"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
tower-http = { version = "0.6", features = ["fs"] }
```

If you feel the need to introduce ANY other tech or third-party dependency, you
MUST ask the human for permission.

## Testing Strategy

The Quest Log project uses a comprehensive testing approach with clear
separation between unit tests and integration tests.

### Test Types

**Unit Tests**

- Located in `src/database.rs` as `database::tests`
- Test individual functions and business logic in isolation
- Run with `cargo test --lib` or `just test`
- Fast feedback for development (10 tests covering core database operations)

**Integration Tests**

- Located in `tests/` directory with semantic names:
  - `database_integration.rs`: End-to-end database operations and schema
    validation
  - `business_logic_integration.rs`: Complex business logic workflows (EXP
    calculations, reward claiming)
  - `web_integration.rs`: Web layer operations (handler logic, static file
    serving)
- Test complete workflows from database through to web responses
- Run with `cargo test --test "*integration"` or `just verify`
- Validate system behavior and data integrity

### Testing Commands

**Core Commands:**

- `just test`: Run unit tests only (fast, isolated)
- `just verify`: Run all integration tests (comprehensive, end-to-end)
- `just check`: Run linting + unit tests + compilation check
- `just lint`: Run clippy and formatting checks

**Direct Cargo Commands:**

- `cargo test --lib`: Unit tests only
- `cargo test --test "*integration"`: All integration tests
- `cargo test`: Run all tests (unit + integration)

### Testing Guidelines

- **Unit Tests**: Test business logic functions, data transformations, and
  isolated components
- **Integration Tests**: Test complete user workflows, database operations, and
  web responses
- **Test Data**: Use in-memory SQLite databases for isolation and speed
- **Coverage**: All public APIs and critical paths must be tested
- **CI/CD**: `just check` provides fast feedback; `just verify` ensures system
  integrity

### File Organization

```
tests/
├── database_integration.rs      # Database layer integration tests
├── business_logic_integration.rs # Business logic workflows  
└── web_integration.rs           # Web handlers and responses
```

## Build Commands

All build, deploy and maintenance commands are centralized in the justfile. Use
`just` commands for all operations.

### Development

- `just dev`: Start development server with hot reload
- `just build`: Build the application for production
- `cargo build`: Direct Rust build (use just build instead)
- `cargo run`: Run the application directly

### Testing

- `just test`: Run unit tests only (fast feedback)
- `just verify`: Run integration tests (comprehensive validation)
- `cargo test --lib`: Direct unit test runner
- `cargo test --test "*integration"`: Direct integration test runner
- `cargo test`: Run all tests (unit + integration)
- `cargo test -- --test-threads=1`: Run tests sequentially (for debugging)
- `cargo test test_name`: Run a specific test function
- `cargo test -- --nocapture`: Run tests with output capture disabled

### Code Quality

- `just lint`: Run all linting and formatting checks
- `cargo clippy`: Lint Rust code for common mistakes and improvements
- `cargo fmt`: Format Rust code according to style guidelines
- `cargo fmt --check`: Check if code is properly formatted
- `just check`: Run type checking and validation

### Database

- `just db-migrate`: Run database migrations
- `just db-reset`: Reset database to initial state
- `just db-seed`: Seed database with test data

### Deployment

- `just deploy`: Deploy to production
- `just docker-build`: Build Docker image
- `just docker-run`: Run in Docker container

## Code Style Guidelines

### Rust Code Style

#### General Principles

- Follow the official Rust style guide (rustfmt defaults)
- Use `cargo fmt` and `cargo clippy` religiously
- Prefer explicit over implicit code
- Avoid unnecessary abstractions - keep it simple and direct
- Use meaningful variable and function names
- Document public APIs with rustdoc comments

#### Naming Conventions

- **Functions**: snake_case, descriptive verbs (e.g., `get_user_data`,
  `validate_input`)
- **Variables**: snake_case, descriptive nouns (e.g., `user_name`, `task_list`)
- **Structs/Enums**: PascalCase (e.g., `QuestItem`, `UserStatus`)
- **Traits**: PascalCase ending with trait name if applicable (e.g., `Display`,
  `FromStr`)
- **Constants**: SCREAMING_SNAKE_CASE (e.g., `MAX_RETRY_COUNT`)
- **Modules**: snake_case (e.g., `database`, `web_handlers`)

#### Imports and Organization

```rust
// Group imports by crate, then std, then external
use std::{
    collections::HashMap,
    sync::Arc,
};

// External crates
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

// Local modules
mod database;
mod models;
mod handlers;

// Use explicit imports, avoid globs except for tests
use database::Connection;
```

#### Error Handling

- Use `Result<T, E>` for operations that can fail
- Define custom error types for domain-specific errors
- Use `?` operator for early returns on errors
- Avoid `unwrap()` and `expect()` in production code
- Log errors appropriately

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Validation error: {0}")]
    Validation(String),
}

fn process_data(data: &str) -> Result<ProcessedData, AppError> {
    let parsed = parse_data(data)?;
    validate_data(&parsed)?;
    Ok(process_data(parsed))
}
```

#### Async Code

- Use `async fn` for asynchronous functions
- Prefer `tokio` for async runtime
- Use `Arc<Mutex<T>>` for shared mutable state
- Avoid blocking operations in async contexts

#### Database (SQLite)

- Use prepared statements to prevent SQL injection
- Validate all user inputs before database operations
- Use transactions for multi-step operations
- Close connections properly

#### Web Handlers

- Keep handlers simple and focused on one responsibility
- Extract business logic into separate functions
- Use appropriate HTTP status codes
- Validate inputs and return meaningful error messages

### HTML/CSS Style

#### HTML Structure

- Use semantic HTML elements (`<main>`, `<section>`, `<article>`, etc.)
- Maintain clean, readable structure
- Use data attributes for datastar interactions
- Keep inline styles minimal, prefer external CSS

#### CSS Guidelines

- Use CSS custom properties (variables) for colors, spacing, etc.
- Follow BEM naming convention for classes
- Keep specificity low and flat
- Use flexbox and grid for layouts
- Mobile-first responsive design
- Avoid !important declarations

```css
:root {
  --primary-color: #007bff;
  --spacing-unit: 1rem;
  --font-size-base: 16px;
}

.btn {
  padding: var(--spacing-unit);
  background-color: var(--primary-color);
}

.btn--primary {
  /* Specific styles */
}
```

#### Datastar Integration

- Use data attributes for reactive behavior
- Keep data-star attributes readable and maintainable
- Prefer server-side rendering with datastar updates
- Document complex interactions

### File Organization

```
/src
  /main.rs          # Application entry point
  /lib.rs           # Library code (if separated)
  /database.rs      # Database operations
  /models.rs        # Data structures
  /handlers/        # HTTP request handlers
    /mod.rs
    /quests.rs
    /parent.rs
    /stats.rs
  /templates/       # HTML templates
  /static/          # CSS, JS, images
    /css/
    /images/
/migrations/       # Database migrations
/tests/            # Integration tests
```

### Testing Guidelines

- Write tests for all public functions
- Use descriptive test names: `test_should_do_something_when_condition`
- Test both success and failure cases
- Use `#[cfg(test)]` modules for test utilities
- Mock external dependencies when possible

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_exp_with_valid_quest() {
        let quest = QuestItem { exp: 10, completed: true };
        assert_eq!(calculate_exp(&quest), 10);
    }

    #[test]
    fn test_calculate_exp_with_incomplete_quest() {
        let quest = QuestItem { exp: 10, completed: false };
        assert_eq!(calculate_exp(&quest), 0);
    }
}
```

### Security Guidelines

- Validate all user inputs
- Use parameterized queries for database operations
- Sanitize HTML output to prevent XSS
- Implement proper CORS headers if needed
- Store sensitive data securely (though this app has no authentication)
- Log security-relevant events

### Performance Guidelines

- Keep database queries efficient
- Use connection pooling
- Cache frequently accessed data
- Minimize allocations in hot paths
- Profile performance-critical code

### Documentation

- Document all public APIs with rustdoc
- Include code examples in documentation
- Keep README updated with setup instructions
- Document complex business logic inline

````rust
/// Calculates the total experience points for a completed quest
///
/// # Arguments
/// * `quest` - The quest item to calculate EXP for
///
/// # Returns
/// The experience points if quest is completed, 0 otherwise
///
/// # Examples
/// ```
/// let quest = QuestItem { exp: 10, completed: true };
/// assert_eq!(calculate_exp(&quest), 10);
/// ```
fn calculate_exp(quest: &QuestItem) -> u32 {
    if quest.completed { quest.exp } else { 0 }
}
````

## Commit Guidelines

- Write clear, concise commit messages
- Use present tense: "Add user authentication" not "Added user authentication"
- Reference issue numbers when applicable
- Keep commits focused on single changes
- Squash fixup commits before merging

## Development Workflow

1. Create a feature branch from main
2. Make changes following these guidelines
3. Run tests: `just test`
4. Run linting: `just lint`
5. Commit with clear messages
6. Push and create pull request
7. Code review and merge

## Code Review Checklist

- [ ] Code follows style guidelines
- [ ] Tests are included and passing
- [ ] No clippy warnings
- [ ] Documentation updated
- [ ] Performance considerations addressed
- [ ] Security best practices followed
- [ ] No unnecessary dependencies added

Remember: Quality over speed. Take time to write clean, maintainable code that
follows these guidelines.
