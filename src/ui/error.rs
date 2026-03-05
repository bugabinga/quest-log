use crate::ui::base::{base_page, PageData};
use maud::html;

pub fn error_page(title: &str, heading: &str, message: &str) -> maud::Markup {
    let body_content = html! {
        div class="error-message" {
            h1 { (heading) }
            p { (message) }
            a href="/" { "← Back to Quest Log" }
        }
    };

    base_page(PageData {
        title: title.to_string(),
        body_content,
        weekday: None,
        signals: None,
        computed: None,
        show_nav: false,
    })
}
