# Session: 2026-03-19 Maud Curly Brace Fix

## Challenges Faced

- **Maud attribute parsing**: The `="..."` shorthand treats `{` as a block
  delimiter, requiring parentheses `=("...")` or raw strings `=(r#"..."#)` to
  preserve literal curly braces
- **Firefox-specific error**: The `GenerateExpression` error only appeared in
  Firefox's console, not Chromium

## Learnings

- Maud attribute value shorthand is limited to simple values; complex values
  with special characters need explicit parentheses wrapping
- Raw string literals `r#"..."#` are cleaner for strings containing both single
  and double quotes
- Console errors may differ between browsers - always test in Firefox when
  Datastar issues are reported
