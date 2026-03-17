use maud::{DOCTYPE, PreEscaped, html};

fn nav_link_class(active_route: Option<&str>, route: &str) -> &'static str {
    let is_active = match active_route {
        Some(r) => r == route,
        None => route == "/",
    };
    if is_active {
        "nav-link nav-link--active"
    } else {
        "nav-link"
    }
}

pub struct PageData {
    pub title: String,
    pub body_content: maud::Markup,
    pub weekday: Option<u8>,
    pub signals: Option<String>,
    pub computed: Option<String>,
    pub show_nav: bool,
    pub active_route: Option<String>,
    pub extra_scripts: Option<String>,
}

#[must_use]
pub fn base_page(data: PageData) -> maud::Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (data.title) }
                link rel="manifest" href="/manifest.json";
                link rel="icon" href="/favicon.png" type="image/png";
                link rel="stylesheet" href="/style.css";
                script type="module" src="/js/datastar.js" {}
                script type="module" src="/js/app.js" {}
                @if let Some(ref extra_scripts) = data.extra_scripts {
                    (PreEscaped(extra_scripts))
                }
            }
            body data-weekday=(data.weekday.map(|w| w.to_string()).unwrap_or_default().as_str()) {
                @if data.show_nav {
                    div id="video-modal" class="video-modal" style="display: none;" {
                        div class="video-overlay" {}
                        div class="video-container" {
                            button class="video-close" data-on:click=[Some(PreEscaped("window.closeVideoModal()".to_string()))] { "×" }
                            video id="intro-video" controls playsinline {
                                source src="/video/trailer.mp4" type="video/mp4";
                            }
                        }
                    }

                    header class="game-header" {
                        nav class="game-header-nav" {
                            a href="/" class=(nav_link_class(data.active_route.as_deref(), "/")) {
                                span class="nav-icon nav-icon--sword" {}
                                span { "Quests" }
                            }
                            a href="/bounty" class=(nav_link_class(data.active_route.as_deref(), "/bounty")) {
                                span class="nav-icon nav-icon--chest" {}
                                span { "Bounty" }
                            }
                            a href="/highscore" class=(nav_link_class(data.active_route.as_deref(), "/highscore")) {
                                span class="nav-icon nav-icon--trophy" {}
                                span { "Highscore" }
                            }
                            a href="/editor" class=(nav_link_class(data.active_route.as_deref(), "/editor")) {
                                span class="nav-icon nav-icon--scroll" {}
                                span { "Editor" }
                            }
                            span class="nav-link nav-link--trailer" data-on:click=[Some(PreEscaped("window.openVideoModal()".to_string()))] {
                                span class="nav-icon nav-icon--crystal" {}
                                span { "Trailer" }
                            }
                        }
                    }
                }

                div id="app" {
                    div class="container" {
                        @if let Some(ref signals) = data.signals {
                            div data-signals=(PreEscaped(signals)) {}
                        }
                        @if let Some(ref computed) = data.computed {
                            div data-computed=(PreEscaped(computed)) {}
                        }
                        div class="notifications" {}
                        (data.body_content)
                    }
                }

                script type="module" src="/js/app.js" {}
            }
        }
    }
}
