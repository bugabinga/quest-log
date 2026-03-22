Update /home/oli/Workspace/quest-log/trunk/AGENTS.md to add a new subsection about snapshot testing for Maud templates.

Add this new section after the "### HTML/CSS" subsection (after line 98) and before "### Datastar + Maud + Axum Integration":

```markdown
### Snapshot Testing (Maud Templates)

- `tests/snapshots/` contains snapshot files showing actual HTML output
- Read these snapshots before modifying Maud templates to understand current output
- `cargo x test` - Run tests including snapshot tests
- `cargo insta review` - Review snapshot changes
- `cargo insta test --accept` - Accept snapshot changes
```

The new content should be inserted between line 98 (end of HTML/CSS section) and line 100 (start of Datastar section). Make sure to preserve the blank line between sections.
