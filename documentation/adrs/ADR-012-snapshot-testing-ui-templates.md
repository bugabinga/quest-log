# ADR-012: Snapshot Testing for UI Templates

## Status

Accepted

## Context

Agents (AI coding assistants) struggle to predict the HTML output of Maud
templates. Maud is a Rust macro DSL that compiles to HTML at compile-time. The
translation from Maud syntax to HTML is not immediately obvious to agents,
leading to errors when modifying templates.

This creates a challenge when agents need to:

- Understand what HTML a Maud template produces
- Modify templates without introducing subtle HTML errors
- Verify that changes don't break existing output

## Decision

We will use insta snapshot tests for all UI templates in `src/ui/` and
`src/ui/fragments/`.

### Approach

1. Store snapshots in `tests/snapshots/`
2. Tests serve as both documentation and regression protection
3. Agents can read snapshot files to understand expected HTML output
4. Each template function gets a corresponding snapshot test

### Usage

- Review snapshots: `cargo insta review`
- Accept changes: `cargo insta test --accept`
- Run tests: `cargo x test`

## Consequences

### Positive

- **Clear HTML documentation**: Snapshots show exact HTML output for all
  templates
- **Automatic regression detection**: Any unintended changes are caught
- **Easier code review**: Visual diffs make template changes obvious
- **Agent-friendly**: AI assistants can read snapshots to understand expected
  output
- **Type-safe alongside**: Complements Maud's compile-time checking with runtime
  verification

### Negative

- **Additional test files**: Snapshots add files to maintain
- **Snapshot updates needed**: When templates intentionally change, snapshots
  must be updated
- **Learning curve**: Team needs to understand insta workflow

## Implementation Notes

### Adding a New Snapshot Test

```rust
#[test]
fn test_quest_item_template() {
    let quest = Quest { /* ... */ };
    let html = quest_item(&quest).into_string();
    insta::assert_snapshot!(html);
}
```

### Reviewing Snapshot Changes

When a template changes:

1. Run `cargo x test` - tests will fail if output differs
2. Run `cargo insta review` to see diffs
3. Accept with `cargo insta test --accept` if changes are intentional

## Related Decisions

- **ADR-007**: Server-Driven UI with Datastar (uses Maud templates)
- **ADR-005**: Core Technology Stack (Maud for HTML generation)

## References

- [insta Documentation](https://insta.rs/)
- [Maud Documentation](https://maud.lambda.xyz/)
- [Snapshot Testing Best Practices](https://insta.rs/docs/quickstart/)
