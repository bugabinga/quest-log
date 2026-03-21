//! SSE response utilities for Datastar handlers.
//!
//! All handlers returning SSE should use the [`sse_response!`] macro
//! to ensure consistent tracing and response construction.

/// Creates an SSE response from a vector of events.
///
/// This macro ensures consistent SSE response creation with tracing
/// for all Datastar handlers.
///
/// # Requirements
///
/// - `$events` must be a `Vec<Event>` (not an iterator or reference)
///
/// # Example
///
/// ```ignore
/// use crate::sse_response;
///
/// let events: Vec<Event> = vec![
///     PatchElements::new(html).into(),
///     PatchSignals::new(json).into(),
/// ];
/// Ok(sse_response!(events))
/// ```
#[macro_export]
macro_rules! sse_response {
    ($events:expr) => {{
        for event in &$events {
            tracing::trace!(?event, "📤 SSE event");
        }
        axum::response::sse::Sse::new(futures::stream::iter(
            $events.into_iter().map(Ok::<_, std::convert::Infallible>),
        ))
    }};
}
