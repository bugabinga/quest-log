# ADR-008: Cargo xtask Pattern for Development CLI

## Status

Accepted

## Context

The project needed a unified way to manage development tasks such as running
tests, formatting code, linting, building containers, and managing assets.
Previously, these tasks were scattered across various methods:

- Direct `cargo` commands with different flags
- Shell scripts for container operations
- Manual steps for asset generation
- Ad-hoc commands remembered by team members

This led to several issues:

- Inconsistent command usage across team members
- Missing feature flags (e.g., forgetting `--features test-utils`)
- Difficulty onboarding new developers
- CI/CD pipelines not aligned with local development
- No single source of truth for how to perform tasks

## Decision Drivers

- **Must have**: Works everywhere Cargo works (cross-platform)
- **Must have**: Single source of truth for all development commands
- **Must have**: CI/CD alignment with local development
- **Should have**: Rust-native implementation (no external build tools)
- **Should have**: Discoverable commands with help text
- **Could have**: Extensibility for new tasks
- **Won't have**: External dependencies beyond Cargo

## Considered Options

### Option A: cargo-xtask Pattern

- **Pros**:
  - Pure Rust, compiled with the project
  - Works everywhere Cargo works
  - Type-safe command parsing with `clap`
  - Can share code with main project (types, utilities)
  - No external tool installation required
  - Integrates naturally with `cargo` ecosystem (`cargo x ...`)
- **Cons**:
  - Requires Rust to run any development command
  - Slight compilation overhead for the xtask binary

### Option B: Make (Makefile)

- **Pros**:
  - Universal, available on most systems
  - Simple syntax for straightforward tasks
  - Parallel execution support
- **Cons**:
  - Shell-dependent, poor Windows support
  - No type safety or argument validation
  - Hard to compose complex logic
  - Requires learning Make syntax

### Option C: Just (justfile)

- **Pros**:
  - Modern alternative to Make
  - Better cross-platform support
  - Good documentation and help generation
- **Cons**:
  - External tool dependency (must install `just`)
  - Team must learn new tool
  - Not as powerful as a real programming language

### Option D: Shell Scripts

- **Pros**:
  - Maximum flexibility
  - No additional tools required
- **Cons**:
  - Poor cross-platform support
  - No argument validation
  - Difficult to maintain and test
  - Code duplication between scripts

### Option E: npm/pnpm Scripts

- **Pros**:
  - Familiar to web developers
  - Good for JS-related tasks
- **Cons**:
  - Requires Node.js for a Rust project
  - Awkward for Rust-specific operations
  - Additional ecosystem dependency

## Decision

We adopted the **cargo-xtask pattern** as documented by
[matklad/cargo-xtask](https://github.com/matklad/cargo-xtask).

The implementation lives in `x/` directory with `x/src/main.rs` providing a
single CLI entry point. All development commands go through `cargo x <command>`.

### Available Commands

| Command                                | Description                                                |
| -------------------------------------- | ---------------------------------------------------------- |
| `cargo x test`                         | Run unit tests with `test-utils` feature enabled           |
| `cargo x verify`                       | Run all tests including integration tests                  |
| `cargo x fmt`                          | Format Rust code (cargo fmt) and JS code (deno fmt)        |
| `cargo x lint`                         | Check formatting, run clippy with `-D warnings`, deno lint |
| `cargo x check`                        | Full check (cargo check with test-utils feature)           |
| `cargo x run [level] [cmd]`            | Run application with RUST_LOG set                          |
| `cargo x watch`                        | Watch for changes and rebuild (cargo-watch)                |
| `cargo x clean`                        | Clean build artifacts and remove local database            |
| `cargo x container build/push/migrate` | Container operations with podman                           |
| `cargo x bundle datastar [version]`    | Bundle datastar JS from CDN                                |
| `cargo x assets`                       | Generate optimized images (favicons, icons)                |
| `cargo x commit validate <file>`       | Validate commit message format                             |
| `cargo x browser [--headed]`           | Run Puppeteer E2E tests                                    |

### Implementation Details

```rust
// x/src/main.rs structure
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "x")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Test { args: Vec<String> },
    Verify { args: Vec<String> },
    // ... other commands
}
```

Key patterns used:

1. **Command delegation**: Each subcommand maps to a dedicated function
2. **Shared utilities**: `run_cargo()` and `run_cmd()` for subprocess execution
3. **Feature flags**: Test commands automatically include
   `--features test-utils`
4. **Validation**: Git state checks for release builds, deno availability checks

## Rationale

The cargo-xtask pattern was chosen because:

1. **Rust-native**: The team is already working in Rust; no new language to
   learn
2. **Zero external dependencies**: Works with just Cargo installed
3. **Type safety**: `clap` provides argument parsing with validation
4. **Discoverability**: `cargo x --help` shows all available commands
5. **CI/CD alignment**: Same commands work locally and in CI
6. **Composability**: Can call other tools (deno, podman, git) from Rust

This pattern is particularly well-suited for Rust projects where the team
already has Rust installed and wants a unified, maintainable approach to
development automation.

## Consequences

### Positive

- **Single source of truth**: All commands documented in one place
- **Onboarding**: New developers can run `cargo x --help` to discover commands
- **Consistency**: Same commands work for all team members
- **CI/CD alignment**: Pipelines use the same `cargo x` commands
- **Type safety**: Clap validates arguments at runtime
- **No external tools**: Only requires Cargo which is already needed
- **Extensibility**: Easy to add new commands as the project grows
- **Feature flag handling**: Automatic inclusion of test-utils feature

### Negative

- **Rust requirement**: Cannot run dev commands without Rust toolchain
  - Mitigation: All developers need Rust anyway for the project
- **Compilation overhead**: xtask binary must compile before running
  - Mitigation: Incremental compilation makes this fast after first build
- **No parallel execution**: Unlike Make, commands run sequentially
  - Mitigation: Individual commands can parallelize internally

### Risks

- **Scope creep**: xtask could grow too large
  - Mitigation: Keep commands focused, delegate to external tools when
    appropriate
- **Dependency on xtask**: If xtask breaks, all dev commands fail
  - Mitigation: Keep xtask simple, well-tested, minimal dependencies

## Implementation Notes

1. **Adding new commands**: Add to `Commands` enum and match in `main()`
2. **Command arguments**: Use `clap` attributes for optional/required args
3. **External tools**: Check availability with `ensure_*()` functions
4. **Error handling**: Use `anyhow::Result` with descriptive error messages
5. **Version**: Injected at compile time via `env!("APP_VERSION")`

### Example: Adding a new command

```rust
// 1. Add to Commands enum
#[derive(Subcommand)]
enum Commands {
    // ... existing commands
    /// New command description
    NewCommand {
        #[arg(long)]
        flag: bool,
    },
}

// 2. Add to main() match
match cli.command {
    // ... existing matches
    Commands::NewCommand { flag } => new_command(flag),
}

// 3. Implement the function
fn new_command(flag: bool) -> Result<()> {
    // Implementation
    Ok(())
}
```

## Related Decisions

- ADR-002: Vendor All Web Resources in `static/` Directory - Uses
  `cargo x bundle`
- ADR-003: Use Tracing for Structured Logging - Log level set via `cargo x run`

## References

- [cargo-xtask pattern](https://github.com/matklad/cargo-xtask) by matklad
- [clap documentation](https://docs.rs/clap/) for argument parsing
- [AGENTS.md](../../AGENTS.md) - Agent instructions requiring `cargo x` usage
