use crate::models::QuestStats;
use crate::ui::base::{PageData, base_page};
use crate::ui::fragments::toggle::QuestDisplay;
use maud::{PreEscaped, html};

pub fn quests_page(
    quests: &[QuestDisplay],
    error_message: &str,
    selected_date: &str,
    day_name: &str,
    is_today: bool,
    class_left: &str,
    class_right: &str,
    can_navigate_left: bool,
    can_navigate_right: bool,
    prev_date: &str,
    next_date: &str,
    weekday_num: u8,
    stats: &QuestStats,
) -> maud::Markup {
    let signals = format!(
        "{{expToday: {}, expTodayMax: {}, weekExp: {}, weekExpMax: {}, questsCompleted: {}, questsTotal: {}, currentDay: {}, isToday: {}}}",
        stats.exp_today,
        stats.exp_today_max,
        stats.week_exp,
        stats.week_exp_max,
        stats.quests_completed,
        stats.quests_total,
        weekday_num,
        if is_today { "true" } else { "false" }
    );

    let computed = "{expTodayPercent: () => Math.round($expToday / Math.max($expTodayMax, 1) * 100), weekExpPercent: () => Math.round($weekExp / Math.max($weekExpMax, 1) * 100)}".to_string();

    let nav_left_onclick = if can_navigate_left {
        Some(PreEscaped(format!(
            "@get('/navigate/{}', {{ headers: {{ 'X-Navigate-Dir': 'prev' }} }})",
            prev_date
        )))
    } else {
        None
    };

    let nav_right_onclick = if can_navigate_right {
        Some(PreEscaped(format!(
            "@get('/navigate/{}', {{ headers: {{ 'X-Navigate-Dir': 'next' }} }})",
            next_date
        )))
    } else {
        None
    };

    let today_onclick = if !is_today {
        Some(PreEscaped("@get('/navigate/today')".to_string()))
    } else {
        None
    };

    let body_content = html! {
        div data-signals=(PreEscaped(&signals)) data-computed=(PreEscaped(&computed)) {}

        div id="day-change-detector" style="display: none;"
            data-on-interval="60000; if ($isToday && new Date().getDay() !== $currentDay) { @get('/navigate/today') }"
            data-on:questlog_simulate_day_change__window="if ($isToday && new Date().getDay() !== $currentDay) { @get('/navigate/today') }"
        {}

        div class="notifications" {}
        h1 class="rainbow-text" { "Quest Log" }

        div class="day-navigation" {
            button class=(class_left) id="nav-left" aria-label="Previous Day"
                data-on:click=[nav_left_onclick] {}
            div class="day-info" id="day-info" {
                h2 class="day-title" { (day_name) }
                p class="day-date" { (selected_date) }
            }
            button class=(class_right) id="nav-right" aria-label="Next Day"
                data-on:click=[nav_right_onclick] {}
        }

        @if !is_today {
            div class="today-button" id="today-btn-container" {
                button class="today-btn" data-on:click=[today_onclick] { "🏰 Return to Today" }
            }
        } @else {
            div class="today-button" id="today-btn-container" style="display: none;" {}
        }

        div class="stats-panel" id="stats-panel" {
            div class="stat-row" {
                span class="stat-label" { "Today's EXP:" }
                span class="stat-value" id="exp-counter" {
                    span data-text=(PreEscaped("$expToday")) {}
                    " / "
                    span data-text=(PreEscaped("$expTodayMax")) {}
                }
            }
            div class="progress-bar" {
                div class="progress-fill" data-style:width=(PreEscaped("$expTodayPercent + '%'")) {}
            }
            div class="stat-row" {
                span class="stat-label" { "Weekly Progress:" }
                span class="stat-value" {
                    span data-text=(PreEscaped("$weekExp")) {}
                    " / "
                    span data-text=(PreEscaped("$weekExpMax")) {}
                    " EXP"
                }
            }
            div class="progress-bar weekly" {
                div class="progress-fill" data-style:width=(PreEscaped("$weekExpPercent + '%'")) {}
            }
            div class="stat-row" {
                span class="stat-label" { "Quests Completed:" }
                span class="stat-value" {
                    span data-text=(PreEscaped("$questsCompleted")) {}
                    " / "
                    span data-text=(PreEscaped("$questsTotal")) {}
                }
            }
        }

        @if !error_message.is_empty() {
            div class="error-message" { (error_message) }
        }

        @if !quests.is_empty() {
            div class="quests-list" id="quest-list" style="view-transition-name: quest-list;" {
                @for quest in quests {
                    div class=(if quest.completed_today { "quest-item completed" } else if quest.is_past { "quest-item past" } else if quest.is_future { "quest-item future" } else { "quest-item" })
                         id=(format!("quest-{}", quest.id))
                         data-quest-id=(quest.id)
                         style=(format!("view-transition-name: quest-{};", quest.id))
                         data-view-transition="quest-morph" {
                        div class="quest-content" {
                            h3 { (quest.title.as_str()) }
                            @if !quest.description.is_empty() {
                                p { (quest.description.as_str()) }
                            }
                            span class="exp-value" { (quest.exp_value) " EXP" }
                        }
                        div class="quest-actions" {
                            @if is_today {
                                button class=(if quest.completed_today { "toggle-btn completed" } else { "toggle-btn" })
                                     type="button"
                                     data-on:click__prevent=[Some(PreEscaped(format!("@post('/quests/toggle', {{ payload: {{ quest_id: {} }} }})", quest.id)))] {
                                    @if quest.completed_today {
                                        "✅ Quest Complete"
                                    } @else {
                                        "⚔️ Mark Complete"
                                    }
                                }
                            } @else if quest.is_past {
                                div class=(if quest.completed_today { "completion-status quest-status--past completed" } else { "completion-status quest-status--past" }) {
                                    @if quest.completed_today {
                                        "✅ Quest Complete"
                                    } @else {
                                        "💀 Quest Failed"
                                    }
                                }
                            } @else {
                                div class=(if quest.completed_today { "completion-status quest-status--future completed" } else { "completion-status quest-status--future" }) {
                                    @if quest.completed_today {
                                        "✅ Quest Complete"
                                    } @else {
                                        "🗺️ Adventure Awaits"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } @else {
            div class="quests-list" id="quest-list" style="view-transition-name: quest-list;" {
                p class="no-quests" { "Your parent forgot to assign quests for this day! Time to create some adventures! 🎮" }
            }
        }
    };

    base_page(PageData {
        title: day_name.to_string(),
        body_content,
        weekday: Some(weekday_num),
        signals: Some(signals),
        computed: Some(computed),
        show_nav: true,
        active_route: Some("/".to_string()),
        extra_scripts: None,
    })
}
