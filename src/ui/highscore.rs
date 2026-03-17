use chrono::NaiveDate;

use crate::handlers::stats::HighscoreData;
use crate::ui::base::{PageData, base_page};
use maud::html;

#[must_use]
pub fn highscore_page(data: HighscoreData) -> maud::Markup {
    let body_content = html! {
        h1 class="rainbow-text" { "🏆 Highscore 🏆" }

        div class="highscore-stats" {
            div class="stat-card" {
                span class="stat-icon" { "⚔️" }
                span class="stat-value" { (data.total_exp) }
                span class="stat-label" { "Total EXP" }
            }

            div class="stat-card" {
                span class="stat-icon" { "✅" }
                span class="stat-value" { (data.quests_completed) }
                span class="stat-label" { "Quests Completed" }
            }

            div class="stat-card" {
                span class="stat-icon" { "🎁" }
                span class="stat-value" { (data.rewards_claimed) }
                span class="stat-label" { "Rewards Claimed" }
            }

            div class="stat-card" {
                span class="stat-icon" { "👑" }
                span class="stat-value" { (data.weekly_champions) }
                span class="stat-label" { "Weekly Champions" }
            }
        }

        h2 { "📜 Quest History" }

        div class="history-list" {
            @if data.completions_by_date.is_empty() {
                p { "No quest completions yet. Start completing quests to see your history!" }
            } @else {
                table class="history-table" {
                    thead {
                        tr {
                            th { "Date" }
                            th { "Quests Completed" }
                        }
                    }
                    tbody {
                        @for (date, count) in &data.completions_by_date {
                            tr {
                                td { (format_date(*date)) }
                                td { (count) }
                            }
                        }
                    }
                }
            }
        }
    };

    base_page(PageData {
        title: "Highscore".to_string(),
        body_content,
        weekday: None,
        signals: None,
        computed: None,
        show_nav: true,
        active_route: Some("/highscore".to_string()),
        extra_scripts: None,
    })
}

fn format_date(date: NaiveDate) -> String {
    use chrono::Datelike;
    let day = date.day();
    let month = match date.month() {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "?",
    };
    let year = date.year();
    format!("{day} {month} {year}")
}
