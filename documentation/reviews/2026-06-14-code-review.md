# Code review 2026-06-14

## Baseline

- `cargo x verify` passed before changes: Rust tests + Chrome e2e, 4 browser tests.

## Issues

### QL-2026-06-14-001 — Login rate limit trusts spoofable forwarding headers

- Category: Security
- Severity: Important
- Location: `src/handlers/editor/auth.rs:33-44`
- Problem: `get_client_ip` uses `X-Forwarded-For` / `X-Real-IP` directly. Any client can rotate those headers and bypass editor login rate limiting.
- Repro: `test_login_rate_limit_ignores_spoofed_forwarded_for` failed red. Sixth wrong password with fresh `X-Forwarded-For` returned `Invalid password` instead of `Too many login attempts`.
- Fix: stop trusting forwarded headers without trusted proxy config. Collapse editor login rate-limit key to the local app client bucket.
- Regression: `tests/editor_integration.rs::test_login_rate_limit_ignores_spoofed_forwarded_for`.

### QL-2026-06-14-002 — Editor Datastar init aborts before login

- Category: Correctness / Testing
- Severity: Critical
- Location: `src/ui/base.rs:101-103`, `src/ui/auth.rs:21-30`
- Problem: editor page passes `Some(String::new())` for computed signals. `base_page` renders `data-computed=""`; Datastar throws `ValueRequired` during page init and stops binding `data-on:*`. Login expression also read unprefixed signals and used Datastar's default JSON content type, while `login_handler` expects form data. After that, successful login patched a full HTML page inside a wrapper instead of navigating to authenticated `/editor`.
- Repro: `tests/e2e/auth.test.ts::Editor login reaches editor UI` failed red in real Chrome; probe showed `pageerror ValueRequired ... plugin computed`, native `POST /editor` → 405, then hidden editor markup after full-page patch.
- Fix: do not render empty `data-computed`; use `$` signal reads/writes; bind password with `data-bind:_password`; post login as `{contentType: 'form'}`; redirect to `/editor` after session cookie is set.
- Regression: `tests/e2e/auth.test.ts::Editor login reaches editor UI`.

### QL-2026-06-14-003 — Editor tabs call missing routes

- Category: Correctness
- Severity: Important
- Location: `src/ui/editor.rs:59-91`
- Problem: tab buttons call `/editor/tab/{name}`, but no such routes exist in `src/main.rs`. Browser clicks do not switch visible panels.
- Repro: `tests/e2e/editor-tabs.test.ts::Editor tabs switch without server errors` failed red in real Chrome waiting for rewards panel visibility.
- Fix: switch tabs client-side by updating the `_activeTab` signal; bind button active classes and panel visibility to that signal.
- Regression: `tests/e2e/editor-tabs.test.ts::Editor tabs switch without server errors`.

### QL-2026-06-14-004 — Editor form signals use invalid Datastar expressions

- Category: Correctness
- Severity: Important
- Location: `src/ui/editor.rs:24-48`, `src/ui/editor.rs:130-330`
- Problem: editor form visibility, validation, and submit expressions reference bare names like `_showQuestForm`. Datastar requires signal reads/writes via `$` (`$_showQuestForm`). After login, Datastar throws `ReferenceError: _showQuestForm is not defined`, aborting later bindings including tab panel visibility.
- Repro: real Chrome probe after login emitted `pageerror ExecuteExpression ... _showQuestForm is not defined`; editor tabs test stayed hidden.
- Fix: prefix editor signal reads/writes with `$`; add computed signals for form validity and labels.
- Regression: `tests/e2e/editor-tabs.test.ts::Editor tabs switch without server errors`; future CRUD e2e should cover add/edit/delete.
