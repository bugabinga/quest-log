use crate::ui::base::{PageData, base_page};
use maud::html;

#[must_use]
pub fn error_page(title: &str, heading: &str, message: &str) -> maud::Markup {
    let body_content = html! {
        div class="error-message" {
            h1 { (heading) }
            p { (message) }
            a href="/" { "← Back to Quest Log" }
        }
    };

    let page_data = PageData {
        title: title.to_string(),
        body_content,
        weekday: None,
        signals: None,
        computed: None,
        show_nav: false,
        active_route: None,
        extra_scripts: None,
    };

    base_page(&page_data)
}
