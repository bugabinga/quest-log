# Session: 2026-03-19 15:30

## What We Achieved

- **Fixed Datastar/Maud syntax errors**: Curly braces `{` in attribute values
  were being misinterpreted by Maud as block delimiters, causing malformed
  JavaScript (`return ({});`)
- **Identified root cause**: Maud's `="value"` shorthand truncates at `{` since
  it denotes a block start
- **Applied fix pattern**: Changed attribute values from `="{'key': value}"` to
  `=("{'key': value}")` or `=(r#"{'key': value}"#)` parenthesized/raw string
  syntax
- **Fixed 7 locations across 3 files**:
  - `src/ui/editor.rs`: 3 `data-class` attributes on tab buttons
  - `src/ui/base.rs`: 2 `data-on-signal-patch-filter` attributes
  - `src/ui/quests.rs`: 2 `data-on-signal-patch-filter` attributes

## Challenges Faced

- **Maud attribute parsing**: The `="..."` shorthand treats `{` as a block
  delimiter, requiring either parentheses `=("...")` or raw strings
  `=(r#"..."#)` to preserve literal curly braces
- **Firefox-specific error**: The `GenerateExpression` error only appeared in
  Firefox's console, not Chromium (likely due to different JS engine error
  messaging)
- **Verification approach**: Delegated to manual-tester subagent to verify in
  Firefox after fixes

## Learnings

- Maud's attribute value shorthand is limited to simple values; complex values
  with special characters need explicit parentheses wrapping
- Raw string literals `r#"..."#` are cleaner for strings containing both single
  and double quotes
- For signals/computed, wrapping JSON in `format!("({})", json)` creates valid
  JavaScript object literal syntax that Datastar expects
- Console errors may differ between browsers - always test in Firefox when
  Datastar issues are reported

## Unfinished / Next Steps

- None - all identified issues have been fixed and verified
