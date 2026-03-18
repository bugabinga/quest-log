//! Server-Sent Events handler for the Quest Log

use std::convert::Infallible;

use axum::extract::State;
use axum::response::sse::{Event, Sse};
use futures::Stream;
use futures::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tracing::instrument;

use crate::handlers::ServerMessage;
use crate::state::AppState;

/// Server-sent events endpoint for real-time updates
#[instrument(name = "📡 events", skip(state))]
pub async fn events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    tracing::debug!("📡 SSE connection opened - client subscribed to updates");
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
