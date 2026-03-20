# Session: 2026-03-20 Editor JS Error Fix

## What We Achieved

- **Fixed `SyntaxError: expected property name, got ')'`** in the editor page -
  the root cause was unescaped quotes in `data-signals` and `data-computed` HTML
  attributes
- **Added `escape_for_html_attr()` helper** in `src/ui/base.rs` to properly
  escape `"` as `&quot;` in attribute values
- **Removed duplicate signal definitions** - signals were being defined both at
  page level (base.rs) and on `.editor-content` div (editor.rs)
- **Removed broken reactive bindings** - `data-class` and `data-show` attributes
  on tabs referenced signals in wrong scope
- **Set up E2E test infrastructure** with Deno + Puppeteer (`tests/e2e/`
  directory with browser helpers and auth tests)
- **Added DOM types to deno.json** (`"lib": ["deno.window", "dom"]`) for
  TypeScript checking

## Challenges Faced

- **Maud `PreEscaped` vs HTML escaping**: `PreEscaped` prevents Maud from
  escaping, but we needed the opposite - to escape quotes for HTML attribute
  context while keeping the content. Solution: create `escape_for_html_attr()`
  that escapes quotes before wrapping in `PreEscaped`.

- **Import order linting**: Rustfmt wants alphabetical imports
  (`{DOCTYPE, PreEscaped, html}` not `{html, PreEscaped, DOCTYPE}`). The edit
  tool sometimes didn't persist changes, requiring `cat > file` workarounds.

- **Server not rebuilding**: Had to manually kill and restart server multiple
  times to pick up code changes.

- **Scope confusion with Datastar signals**: The tabs used `data-class`
  referencing `_activeTab`, but signals were on a sibling div
  (`.editor-content`), not a parent. Datastar signals scope is determined by
  closest ancestor with `data-signals`.

## Learnings

1. **Maud attribute escaping is tricky**: Use `=(value)` for escaped content,
   `={value}` for unescaped interpolation. For HTML attributes containing
   JSON/JS, you must manually escape `"` to `&quot;`.

2. **Datastar signals require proper DOM hierarchy**: Child elements can access
   signals from ancestors, but siblings cannot. If you need shared state, put
   `data-signals` on a common parent.

3. **Test-first approach helped catch the fix**: The E2E test "No JavaScript
   errors on page load" passed after the fix, confirming the solution.

4. **Manual testing is essential**: The automated tests passed, but manual
   testing revealed the login was broken (HTTP 405) - a pre-existing issue
   unrelated to our changes.

## Unfinished / Next Steps

- **HTTP 405 on login endpoint** - Login returns "Method Not Allowed",
  preventing editor access. This is a pre-existing bug.

- **Submit button not disabled when empty** - The auth modal lacks
  `data-attr:disabled` reactive binding. Never implemented.

- **`_showQuestForm is not defined` error** - Signals scope issue in quests
  panel. Pre-existing bug.

- **Computed signals disabled** - We emptied the computed signals to avoid
  circular dependency errors. Should investigate proper Datastar computed signal
  usage.
