use maud::{DOCTYPE, PreEscaped, html};

pub struct PageData {
    pub title: String,
    pub body_content: maud::Markup,
    pub weekday: Option<u8>,
    pub signals: Option<String>,
    pub computed: Option<String>,
    pub show_nav: bool,
}

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
                            a href="/" class="nav-link nav-link--active" {
                                span class="nav-icon nav-icon--sword" {}
                                span { "Quests" }
                            }
                            a href="/highscore" class="nav-link" {
                                span class="nav-icon nav-icon--trophy" {}
                                span { "Highscore" }
                            }
                            a href="/editor" class="nav-link" {
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
