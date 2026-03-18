use maud::{Markup, PreEscaped, html};

/// Renders navigation buttons for moving between days.
pub fn nav_buttons(class: &str, can_navigate: bool, target_date: &str) -> Markup {
    let id = if class.contains("left") {
        "nav-left"
    } else {
        "nav-right"
    };
    let label = if class.contains("left") {
        "Previous Day"
    } else {
        "Next Day"
    };
    let dir = if class.contains("left") {
        "prev"
    } else {
        "next"
    };
    let onclick = if can_navigate {
        Some(format!(
            "@get('/navigate/{target_date}', {{ headers: {{ 'X-Navigate-Dir': '{dir}' }} }})"
        ))
    } else {
        None
    };

    html! {
        button class=(class) id=(id) aria-label=(label)
            data-on:click=[onclick.map(PreEscaped)] {}
    }
}
