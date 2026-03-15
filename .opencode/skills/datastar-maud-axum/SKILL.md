---
name: datastar-maud-axum
description: Guide for using Datastar, Maud, and Axum together in Rust web applications — patch elements, update signals, integrate with Maud templates, and handle server-sent events.
---

## What I do

This skill teaches agents how to properly integrate Datastar (hypermedia
framework), Maud (HTML templating), and Axum (web framework) to build reactive
web applications in the Quest Log codebase.

## Core Concepts

### Datastar Fundamentals

Datastar enables backend-driven interactivity through:

- Server-Sent Events (SSE) for pushing updates from backend to frontend
- `data-*` attributes for declarative frontend behavior
- Patching elements into the DOM via morphing strategy
- Updating reactive signals without full page reloads

**Key Principles (from Datastar Guide):**

- Backend is the source of truth - most state lives on the server
- Use signals sparingly - only for user interactions and form bindings
- Send "fat morphs" - large DOM chunks rather than fine-grained updates
- Default configuration works well - don't change unless necessary
- Use CQRS pattern: SSE for reads, short requests for writes

### Maud Integration

Maud provides compile-time verified HTML templates:

- `html!` macro for type-safe HTML generation
- Components as functions returning `maud::Markup`
- Seamless integration with Axum handlers
- Automatic escaping prevents XSS vulnerabilities
- Minimal runtime (~100 SLoC with framework integrations)

### Axum Handler Patterns

Handlers return SSE streams for Datastar communication:

- Use `ReadSignals` extractor to get client-sent data
- Return `Sse<impl Stream<Item = Result<Event, Infallible>>>`
- Patch elements and signals via Datastar actions
- Broadcast updates to all connected clients when needed

## Implementation Patterns

### Handler with Path and Query Parameters

```rust
use axum::{
    extract::{Path, Query, State},
    response::sse::{Event, Sse},
};
use datastar::axum::ReadSignals;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Signals {
    pub status: String,
}

async fn status_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ReadSignals(signals): ReadSignals<Signals>,
) -> impl IntoResponse {
    // Handle request with path param and signals
    Sse::new(stream! { /* ... */ })
}
```

### Handler with Combined PatchElements + PatchSignals

From axum-activity-feed.rs - append to feed and update signals:

```rust
use datastar::{patch_elements::PatchElements, patch_signals::PatchSignals};
use datastar::prelude::ElementPatchMode;

// First: Update signals (counters, flags)
let signal_patch = PatchSignals::new(r#"{"generating": true}"#);
let sse_event = signal_patch.write_as_axum_sse_event();
yielder.yield_item(Ok(sse_event)).await;

// Then: Append element to feed
let elements_patch = PatchElements::new(elements)
    .selector("#feed")
    .mode(ElementPatchMode::After);
let sse_event = elements_patch.write_as_axum_sse_event();
yielder.yield_item(Ok(sse_event)).await;
```

### ExecuteScript for Client-Side JavaScript

```rust
use datastar::prelude::ExecuteScript;

let script = ExecuteScript::new("window.location.reload()");
let sse_event = script.write_as_axum_sse_event();
```

### Handler with Status Enum

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Done,
    Warn,
    Fail,
    Info,
}

async fn event_handler(
    Path(status): Path<Status>,
    // Route: /event/done, /event/warn, etc.
) -> impl IntoResponse { /* ... */ }
```

### Basic Handler Structure

```rust
use axum::{
    extract::{State, ReadSignals},
    response::sse::{Event, Sse},
};
use datastar::{patch_elements::PatchElements, patch_signals::PatchSignals};
use futures::stream::{self, Stream};
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;

// In your handler function
async fn my_handler(
    State(state): State<AppState>,
    ReadSignals(payload): ReadSignals<MyPayload>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    // Process request and generate HTML with Maud
    let html = ui::fragments::my_component(&data).into_string();
    
    // Generate signals JSON
    let signals_json = serde_json::json!({
        "key": "value"
    });
    
    // Create Datastar events
    let events = vec![
        PatchElements::new(html)
            .use_view_transition(true) // Optional: enables smooth transitions
            .into(),
        PatchSignals::new(signals_json.to_string()).into(),
    ];
    
    let stream = stream::iter(events.into_iter().map(Ok));
    Ok(Sse::new(stream))
}
```

### Deriving from base.rs

The `src/ui/base.rs` file provides the foundation for all pages. Use it as the
base layout and inject your page-specific content.

#### PageData Structure

```rust
pub struct PageData {
    pub title: String,           // Page title
    pub body_content: maud::Markup,  // Your page content
    pub weekday: Option<u8>,    // Day of week (0-6)
    pub signals: Option<String>,  // Datastar signals JSON
    pub computed: Option<String>, // Computed signals (JS expressions)
    pub show_nav: bool,          // Show/hide navigation
    pub active_route: Option<String>, // Current route for nav highlighting
    pub extra_scripts: Option<String>, // Additional JS to include
}
```

#### Creating a New Page

1. **Create your page function** that builds body content:

```rust
use crate::ui::base::{PageData, base_page};
use maud::html;

pub fn my_page(content: &MyData) -> maud::Markup {
    // Build your page-specific content
    let body_content = html! {
        h1 { "My Page Title" }
        div class="my-content" {
            // Page-specific elements with data-* attributes
        }
    };
    
    // Create signals for frontend state
    let signals = serde_json::json!({
        "count": 0,
        "isLoading": false
    }).to_string();
    
    // Wrap with base page
    base_page(PageData {
        title: "My Page".to_string(),
        body_content,
        weekday: None,
        signals: Some(signals),
        computed: None,
        show_nav: true,
        active_route: Some("/my-page".to_string()),
        extra_scripts: None,
    })
}
```

2. **Pass signals to body_content** (if needed):

```rust
use maud::PreEscaped;

let body_content = html! {
    // Signals available to all children
    div data-signals=(PreEscaped(&signals)) {}
    
    // Your content with data-* attributes
    div id="my-element" {
        span data-text="$count" {}
    }
};
```

3. **Add computed signals** for derived state:

```rust
let computed = r#"{countDoubled: () => $count * 2}"#.to_string();

base_page(PageData {
    // ...
    signals: Some(signals),
    computed: Some(computed),
    // ...
})
```

#### Example: quests_page derivation

From `src/ui/quests.rs`:

```rust
pub fn quests_page(/* params */) -> maud::Markup {
    // Build signals from stats
    let signals = format!(
        "{{expToday: {}, expTodayMax: {}, weekExp: {}, ...}}",
        stats.exp_today, stats.exp_today_max, stats.week_exp
    );
    
    // Build computed signals
    let computed = "{expTodayPercent: () => Math.round($expToday / Math.max($expTodayMax, 1) * 100)}".to_string();
    
    // Build body with signals/computed as data attributes
    let body_content = html! {
        div data-signals=(PreEscaped(&signals)) data-computed=(PreEscaped(&computed)) {}
        
        // Page-specific content with data-* attributes
        h1 class="rainbow-text" { "Quest Log" }
        // ...
    };
    
    base_page(PageData {
        title: day_name.to_string(),
        body_content,
        weekday: Some(weekday_num),
        signals: Some(signals),
        computed: Some(computed),
        show_nav: true,
        active_route: Some("/".to_string()),
        extra_scripts: None,
    })
}
```

#### Key Patterns

1. **Always wrap content with `base_page()`** - provides Datastar scripts, CSS,
   navigation
2. **Pass signals in body_content** - so nested elements can access them via
   data attributes
3. **Use PreEscaped for signals/computed** - they're already formatted as
   JS/JSON
4. **Set active_route** - highlights current nav item
5. **Use view-transition-name** - enables smooth animations on element changes

### Component Creation with Maud

#### Basic Maud Template

```rust
use maud::{html, Markup};

pub fn greeting(name: &str) -> Markup {
    html! {
        p { "Hi, " (name) "!" }
    }
}
```

#### Elements with Attributes

```rust
html! {
    ul {
        li {
            a href="about:blank" { "Link" }
        }
        li class="active" {
            "Active item"
        }
        li dir="rtl" {
            "Right-to-left"
        }
    }
}
```

#### Classes and IDs Shortcut

```rust
// Generates: <input id="cannon" class="big scary bright-red" type="button" value="Launch">
input #cannon .big.scary.bright-red type="button" value="Launch";
```

#### Implicit div

```rust
html! {
    #main { "Main content!" }
    .tip { "Pro tip here" }
}
```

#### Control Flow

```rust
// @if / @else
@if user.is_active {
    span.active { "Active" }
} @else {
    span { "Inactive" }
}

// @if let
@if let Some(name) = user.name {
    p { "Hello, " (name) }
}

// @for loop
@for quest in &quests {
    li { (quest.title) }
}

// @let variable
@let first_letter = name.chars().next();
li { (first_letter) ". " (name) }

// @match pattern matching
@match status {
    Status::Active => { span { "Active" } },
    _ => { span { "Unknown" } }
}
```

#### Render Trait Implementation

```rust
use maud::{html, Markup, Render};

pub struct Button {
    pub label: String,
    pub onclick: String,
}

impl Render for Button {
    fn render(&self) -> Markup {
        html! {
            button onclick=(self.onclick) { (self.label) }
        }
    }
}

// Usage
html! { (Button { label: "Click me".into(), onclick: "alert('hi')".into() }) }
```

#### PreEscaped (Raw HTML)

```rust
use maud::PreEscaped;

let raw_html = "<p>Raw content</p>";
html! {
    h1 { "Title" }
    (PreEscaped(raw_html))  // Not escaped!
}
```

#### DOCTYPE

```rust
use maud::DOCTYPE;

html! {
    (DOCTYPE)
    html lang="en" { /* ... */ }
}
```

#### Layout Pattern

```rust
fn header(title: &str) -> Markup {
    html! {
        meta charset="utf-8";
        title { (title) }
    }
}

fn page(title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head { (header(title)) }
            body { (content) }
        }
    }
}
```

### Component Creation with Maud

```rust
use maud::{html, Markup};

pub fn my_component(data: &MyData) -> Markup {
    html! {
        div class="my-component" data-id=(data.id) {
            h2 { (data.title) }
            p { (data.description) }
            @if data.is_active {
                span class="status-active" { "Active" }
            } @else {
                span class="status-inactive" { "Inactive" }
            }
        }
    }
}
```

### Broadcasting Updates

When you need to update all connected clients (e.g., after a mutation):

```rust
// In your AppState or handler
if let Err(e) = bcast.send(ServerMessage::Elements(html_clone, origin)) {
    tracing::error!(error = %e, "💥 Failed to broadcast elements");
}
if let Err(e) = bcast.send(ServerMessage::Signals(signals_json.to_string(), origin)) {
    tracing::error!(error = %e, "💥 Failed to broadcast signals");
}
```

### SSE Event Handler

The events endpoint translates internal messages to Datastar SSE format:

```rust
pub async fn events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.bcast.subscribe();

    let stream = async_stream::stream! {
        let mut rx = BroadcastStream::new(rx);
        while let Some(res) = rx.next().await {
            match res {
                Ok(ServerMessage::Elements(html, origin)) => {
                    let payload = serde_json::json!({
                        "data": html,
                        "origin": origin
                    });
                    let ev = Event::default()
                        .event("datastar-patch-elements")
                        .data(payload.to_string());
                    yield Ok(ev);
                }
                Ok(ServerMessage::Signals(json, origin)) => {
                    let payload = serde_json::json!({
                        "data": json,
                        "origin": origin
                    });
                    let ev = Event::default()
                        .event("datastar-patch-signals")
                        .data(payload.to_string());
                    yield Ok(ev);
                }
                Err(_) => {}
            }
        }
    };

    Sse::new(stream)
}
```

## Datastar Frontend Patterns

### Data Attributes Reference

| Attribute              | Purpose                    |
| ---------------------- | -------------------------- |
| `data-bind:*`          | Two-way binding to signals |
| `data-signals:*`       | Define/initialize signals  |
| `data-text`            | Text content binding       |
| `data-computed:*`      | Computed/derived signals   |
| `data-show`            | Conditional visibility     |
| `data-class:*`         | Dynamic classes            |
| `data-attr:*`          | Dynamic attributes         |
| `data-on:*`            | Event handlers             |
| `data-on-intersect`    | Intersection Observer      |
| `data-on-signal-patch` | React to signal changes    |
| `data-effect`          | Side effects               |
| `data-indicator:*`     | Loading state              |
| `data-init`            | Initial SSE connection     |

### Common Frontend Patterns

#### Click to Edit

```html
<!-- View mode -->
<div id="item">
  <p>Name: John</p>
  <button data-on:click="@get('/edit/1')">Edit</button>
</div>

<!-- Edit mode (returned by server) -->
<div id="item">
  <input type="text" data-bind:name>
  <button data-on:click="@put('/update/1')">Save</button>
  <button data-on:click="@get('/view/1')">Cancel</button>
</div>
```

#### Debounced Search

```html
<input
  type="text"
  placeholder="Search..."
  data-bind:search
  data-on:input__debounce.200ms="@get('/search')"
/>
<div id="results"></div>
```

#### Loading Indicator

```html
<button
  data-on:click="@post('/action')"
  data-indicator:fetching
>
  Submit
</button>
<div data-class:loading="$fetching">Loading...</div>
```

#### Form Submission

```html
<form id="myform">
  <input type="checkbox" name="options" value="foo" />
  <button data-on:click="@post('/endpoint', {contentType: 'form'})">
    Submit
  </button>
</form>
```

#### Conditional Visibility

```rust
// Server sets initial style to prevent flash
<button data-show="$isVisible" style="display: none">Save</button>
```

### SSE Event Formats

#### Patch Elements

```http
event: datastar-patch-elements
data: useViewTransition true
data: selector #my-element
data: mode outer
data: elements <div id="my-element">Content</div>
```

#### Patch Signals

```http
event: datastar-patch-signals
data: signals {"count": 5, "name": "test"}
```

#### Execute Script

```http
event: datastar-patch-elements
data: elements <script>alert('Hello')</script>
```

## Best Practices

1. **Always use `view-transition:true`** for PatchElements when updating UI
   components for smooth animations
2. **Separate concerns**: Keep HTML generation in Maud components, business
   logic in handlers
3. **Type-safe signals**: Use structs with Serde Deserialize for ReadSignals
   extractor
4. **Error handling**: Return AppError from handlers, let IntoResponse
   implementation convert to proper HTTP responses
5. **Performance**: Only patch elements that actually changed, avoid full page
   replacements when possible
6. **Security**: Maud automatically escapes content, but be cautious with
   PreEscaped - only use trusted content
7. **Loading states**: Consider adding skeleton UI or loading indicators during
   data fetching
8. **Accessibility**: Ensure patched content maintains proper ARIA labels and
   semantic structure
9. **Patch modes**: Use appropriate merge modes (outer, inner, prepend, append,
   before, after, remove)
10. **Origin filtering**: Always set origin in broadcasts for client filtering
11. **Streaming**: Use `stream!` macro for multiple events over time,
    `stream::iter` for single batch

## Patch Modes Reference

| Mode              | Use Case               |
| ----------------- | ---------------------- |
| `outer` (default) | Replace entire element |
| `inner`           | Replace inner content  |
| `prepend`         | Add as first child     |
| `append`          | Add as last child      |
| `before`          | Insert before element  |
| `after`           | Insert after element   |
| `remove`          | Delete element         |

## Streaming vs Simple Responses

**Simple responses** (single batch updates) - use `stream::iter`:

```rust
let events = vec![quest_patch.into(), signals_patch.into()];
let stream = stream::iter(events.into_iter().map(Ok));
Ok(Sse::new(stream))
```

**Streaming responses** (multiple events over time) - use
`async_stream::stream!`:

```rust
Sse::new(stream! {
    for i in 0..10 {
        let patch = PatchElements::new(format!("<div id='counter'>{}</div>", i));
        yield patch.into();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
})
```

## Signal Types Pattern

Extract shared signal types to a module for consistency:

```rust
// src/signals.rs
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct QuestSignals {
    pub exp_today: i32,
    pub exp_today_max: i32,
    pub week_exp: i32,
    pub week_exp_max: i32,
    pub quests_completed: i32,
    pub quests_total: i32,
    pub current_day: i32,
    pub is_today: bool,
}
```

## Error Handling for SSE

Yield error elements instead of HTTP errors for better UX:

````rust
async fn safe_handler(
    State(state): State<AppState>,
    ReadSignals(request): ReadSignals<MyRequest>,
) -> impl IntoResponse {
    match try_operation(state, request).await {
        Ok(events) => Sse::new(stream::iter(events.into_iter().map(Ok))),
        Err(e) => {
            let error_html = ui::error_page("Error", "Operation Failed", e.message());
            Sse::new(stream::iter(vec![Ok(PatchElements::new(error_html.into_string()).into())]))
        }
    }
}

## Common Patterns in Quest Log

Looking at the existing codebase, these patterns are already established:

1. **Handler returns SSE stream** with PatchElements and PatchSignals events
2. **Maud components** in `src/ui/fragments/` return `maud::Markup`
3. **Broadcast mechanism** via `state.bcast` for updating all clients
4. **Signals JSON** contains frontend state that needs updating
5. **View transitions** enabled for smooth UI updates

## When to Use This Skill

Use this skill when:
- Building new features that require real-time UI updates
- Creating interactive components that respond to user actions
- Implementing mutation endpoints that should update multiple parts of the UI
- Adding new endpoints that need to integrate with the existing Datastar/Maud/Axum stack
- Refactoring existing handlers to follow established patterns

## Dependencies

Ensure these dependencies are in your Cargo.toml:
```toml
datastar = { version = "*", features = ["axum"] }
maud = { version = "*", features = ["router"] }
axum = { version = "*", features = ["json"] }
serde = { version = "*", features = ["derive"] }
tokio = { version = "*", features = ["stream", "macros", "time"] }
futures = { version = "*" }
async-stream = { version = "*" }
tokio-stream = { version = "*" }
tracing = { version = "*" }
tracing-subscriber = { version = "*", features = ["fmt", "env-filter"] }
````

## Example: Complete Implementation Flow

1. **Create Maud component** in `src/ui/fragments/new_feature.rs`
2. **Add route** in `src/handlers.rs` or appropriate handler module
3. **Implement handler function** following the pattern above
4. **Update AppState** if needed to support broadcasting
5. **Add any new signals** that frontend needs to know about
6. **Test** that updates properly propagate to connected clients

## Reference Implementations in Quest Log

- `src/handlers.rs`: toggle_quest, navigate, claim_reward functions
- `src/handlers/editor/mod.rs`: editor handlers showing editor-specific patterns
- `src/ui/base.rs`: base_page layout showing Datastar script includes
- `src/ui/fragments/`: Individual Maud components

## Verification

After implementing a feature using this skill:

1. Check that Maud templates compile without errors
2. Verify that handlers return proper SSE streams
3. Confirm that PatchElements and PatchSignals are used correctly
4. Test that updates appear in browser without full page reload
5. Ensure error handling returns appropriate HTTP status codes
6. Validate that broadcasting works for multi-client scenarios
