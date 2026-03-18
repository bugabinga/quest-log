use maud::{Markup, PreEscaped, html};

/// Renders a button to navigate back to today's date.
pub fn today_button(is_today: bool) -> Markup {
    let onclick: Option<String> = if is_today {
        None
    } else {
        Some("@get('/navigate/today')".to_string())
    };

    html! {
        @if !is_today {
            div class="today-button" id="today-btn-container" {
                button class="today-btn" data-on:click=[onclick.map(PreEscaped)] { "🏰 Return to Today" }
            }
        } @else {
            div class="today-button" id="today-btn-container" style="display: none;" {}
        }
    }
}
