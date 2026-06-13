# Session: 2026-03-20 Editor JS Error Fix

## Challenges Faced

- **Maud `PreEscaped` vs HTML escaping**: `PreEscaped` prevents Maud from
  escaping, but we needed to escape quotes for HTML attribute context. Solution:
  create `escape_for_html_attr()` that escapes quotes before wrapping in
  `PreEscaped`.
- **Scope confusion with Datastar signals**: Child elements can access signals
  from ancestors, but siblings cannot. If you need shared state, put
  `data-signals` on a common parent.

## Learnings

- **Maud attribute escaping**: Use `=(value)` for escaped content, `={value}`
  for unescaped interpolation. For HTML attributes containing JSON/JS, you must
  manually escape `"` to `&quot;`.
- **Datastar signals require proper DOM hierarchy**: Sibling elements cannot
  share signals unless they have a common ancestor with `data-signals`.
