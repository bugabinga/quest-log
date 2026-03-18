use maud::html;

/// Renders the day header showing the day name and selected date.
#[must_use]
pub fn day_header(day_name: &str, selected_date: &str) -> maud::Markup {
    html! {
        div class="day-info" id="day-info" {
            h2 class="day-title" { (day_name) }
            p class="day-date" { (selected_date) }
        }
    }
}
