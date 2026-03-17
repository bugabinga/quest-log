//! Editor page UI for the Quest Log

use crate::models::{Quest, Reward, Settings};
use crate::ui::auth::auth_modal;
use crate::ui::base::{PageData, base_page};
use maud::{Markup, PreEscaped, html};

/// Day names for the editor dropdown
const DAY_NAMES: &[&str] = &[
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Generate the full editor page HTML
#[must_use]
pub fn editor_page(
    is_authenticated: bool,
    active_tab: &str,
    quests: &[Quest],
    rewards: &[Reward],
    settings: &Settings,
) -> Markup {
    let signals = format!("{{_activeTab: '{active_tab}'}}");

    let body_content = html! {
        div class="editor-wrapper" {
            div class="editor-header" {
                h1 { "🗝️ Quest Log Editor" }
                (logout_button())
            }

            @if is_authenticated {
                div class="editor-container" {
                    div class="editor-tabs" {
                        button
                            class=(if active_tab == "quests" { "editor-tab editor-tab--active" } else { "editor-tab" })
                            data-on:click="_activeTab = 'quests'; @get('/editor/tab/quests')"
                            data-class="{'editor-tab--active': _activeTab === 'quests'}" {
                            "📜 Quests"
                        }
                        button
                            class=(if active_tab == "rewards" { "editor-tab editor-tab--active" } else { "editor-tab" })
                            data-on:click="_activeTab = 'rewards'; @get('/editor/tab/rewards')"
                            data-class="{'editor-tab--active': _activeTab === 'rewards'}" {
                            "🎁 Rewards"
                        }
                        button
                            class=(if active_tab == "settings" { "editor-tab editor-tab--active" } else { "editor-tab" })
                            data-on:click="_activeTab = 'settings'; @get('/editor/tab/settings')"
                            data-class="{'editor-tab--active': _activeTab === 'settings'}" {
                            "⚙️ Settings"
                        }
                    }

                    div class="editor-content" data-signals=(PreEscaped(&signals)) {
                        // Quests Tab
                        div class=(if active_tab == "quests" { "editor-panel" } else { "editor-panel hidden" }) data-show="_activeTab === 'quests'" {
                            (editor_quests_panel(quests))
                        }

                        // Rewards Tab
                        div class=(if active_tab == "rewards" { "editor-panel" } else { "editor-panel hidden" }) data-show="_activeTab === 'rewards'" {
                            (editor_rewards_panel(rewards))
                        }

                        // Settings Tab
                        div class=(if active_tab == "settings" { "editor-panel" } else { "editor-panel hidden" }) data-show="_activeTab === 'settings'" {
                            (editor_settings_panel(settings))
                        }
                    }
                }
            } @else {
                (auth_modal())
            }
        }

        // Toast container for notifications
        div id="toast-container" class="toast-container" {}
    };

    base_page(PageData {
        title: "Quest Log Editor 🗝️".to_string(),
        body_content,
        weekday: None,
        signals: Some(signals),
        computed: None,
        show_nav: true,
        active_route: Some("/editor".to_string()),
        extra_scripts: None,
    })
}

/// Quests panel for the editor
fn editor_quests_panel(quests: &[Quest]) -> Markup {
    html! {
        div id="quests-panel" {
            div class="panel-header" {
                h2 { "📜 Quest Management" }
                button
                    class="editor-btn editor-btn--primary"
                    data-on:click="_showQuestForm = true" {
                    "+ Add Quest"
                }
            }

            // Add/Edit Quest Form
            div id="quest-form-container" class="editor-form-container hidden" data-show="_showQuestForm" {
                (PreEscaped(r#"<form id="quest-form" class="editor-form" action="/editor/quests" method="POST" enctype="multipart/form-data" data-on:submit__prevent="return handleQuestForm(event)">"#))
                    h3 { "Add New Quest" }

                    div class="form-row" {
                        div class="form-group" {
                            label for="quest-title" { "Title *" }
                            input
                                type="text"
                                id="quest-title"
                                name="title"
                                required
                                placeholder="Quest title...";
                        }
                        div class="form-group" {
                            label for="quest-exp" { "EXP Value" }
                            input
                                type="number"
                                id="quest-exp"
                                name="exp_value"
                                value="10"
                                min="0";
                        }
                    }

                    div class="form-row" {
                        div class="form-group" {
                            label for="quest-day" { "Day of Week" }
                            select id="quest-day" name="day_of_week" {
                                @for (i, name) in DAY_NAMES.iter().enumerate() {
                                    option value=(i) { (name) }
                                }
                            }
                        }
                        div class="form-group" {
                            label for="quest-image" { "Image (optional)" }
                            input
                                type="file"
                                id="quest-image"
                                name="image"
                                accept="image/*";
                        }
                    }

                    div class="form-group" {
                        label for="quest-description" { "Description" }
                        textarea
                            id="quest-description"
                            name="description"
                            rows="3"
                            placeholder="Quest description..." {}
                    }

                    div class="form-actions" {
                        button
                            type="button"
                            class="editor-btn editor-btn--secondary"
                            data-on:click="_showQuestForm = false" {
                            "Cancel"
                        }
                        button
                            type="submit"
                            class="editor-btn editor-btn--primary"
                            data-indicator="#quest-saving" {
                            span { "Save Quest" }
                            span id="quest-saving" style="display: none" { "Saving..." }
                        }
                    }
                (PreEscaped("</form>"))
            }

            // Quests Table
            table class="editor-table" {
                thead {
                    tr {
                        th { "Title" }
                        th { "EXP" }
                        th { "Day" }
                        th { "Status" }
                        th { "Actions" }
                    }
                }
                tbody {
                    @for quest in quests {
                        tr class=(if quest.is_active { "" } else { "inactive-row" }) {
                            td { (quest.title) }
                            td { (quest.exp_value) }
                            td { (DAY_NAMES[usize::try_from(quest.day_of_week).unwrap_or(0)]) }
                            td {
                                @if quest.is_active {
                                    span class="status-badge status-badge--active" { "Active" }
                                } @else {
                                    span class="status-badge status-badge--inactive" { "Inactive" }
                                }
                            }
                            td {
                                div class="action-buttons" {
                                    button
                                        class="editor-btn editor-btn--small"
                                        data-on:click="alert('Edit feature coming soon!')" {
                                        "Edit"
                                    }
                                    button
                                        class="editor-btn editor-btn--danger"
                                        data-on:click=[Some(PreEscaped(format!("@delete('/editor/quests/{}')", quest.id)))] {
                                        "Delete"
                                    }
                                }
                            }
                        }
                    }
                    @if quests.is_empty() {
                        tr {
                            td colspan="5" class="empty-message" {
                                "No quests yet. Add your first quest!"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Rewards panel for the editor
fn editor_rewards_panel(rewards: &[Reward]) -> Markup {
    html! {
        div id="rewards-panel" {
            div class="panel-header" {
                h2 { "🎁 Reward Management" }
                button
                    class="editor-btn editor-btn--primary"
                    data-on:click="_showRewardForm = true" {
                    "+ Add Reward"
                }
            }

            // Add/Edit Reward Form
            div id="reward-form-container" class="editor-form-container hidden" data-show="_showRewardForm" {
                (PreEscaped(r#"<form id="reward-form" class="editor-form" action="/editor/rewards" method="POST" enctype="multipart/form-data" data-on:submit__prevent="return handleRewardForm(event)">"#))
                    h3 { "Add New Reward" }

                    div class="form-row" {
                        div class="form-group" {
                            label for="reward-title" { "Title *" }
                            input
                                type="text"
                                id="reward-title"
                                name="title"
                                required
                                placeholder="Reward title...";
                        }
                        div class="form-group" {
                            label for="reward-exp" { "Required EXP *" }
                            input
                                type="number"
                                id="reward-exp"
                                name="required_exp"
                                required
                                min="0"
                                value="50"
                                placeholder="50";
                        }
                    }

                    div class="form-group" {
                        label for="reward-image" { "Image (optional)" }
                        input
                            type="file"
                            id="reward-image"
                            name="image"
                            accept="image/*";
                    }

                    div class="form-group" {
                        label for="reward-description" { "Description" }
                        textarea
                            id="reward-description"
                            name="description"
                            rows="3"
                            placeholder="Reward description..." {}
                    }

                    div class="form-actions" {
                        button
                            type="button"
                            class="editor-btn editor-btn--secondary"
                            data-on:click="_showRewardForm = false" {
                            "Cancel"
                        }
                        button
                            type="submit"
                            class="editor-btn editor-btn--primary"
                            data-indicator="#reward-saving" {
                            span { "Save Reward" }
                            span id="reward-saving" style="display: none" { "Saving..." }
                        }
                    }
                (PreEscaped("</form>"))
            }

            // Rewards Table
            table class="editor-table" {
                thead {
                    tr {
                        th { "Title" }
                        th { "Required EXP" }
                        th { "Status" }
                        th { "Actions" }
                    }
                }
                tbody {
                    @for reward in rewards {
                        tr class=(if reward.is_active { "" } else { "inactive-row" }) {
                            td { (reward.title) }
                            td { (reward.required_exp) }
                            td {
                                @if reward.is_active {
                                    span class="status-badge status-badge--active" { "Active" }
                            } @else {
                                span class="status-badge status-badge--inactive" { "Inactive" }
                            }
                            }
                            td {
                                div class="action-buttons" {
                                    button
                                        class="editor-btn editor-btn--small"
                                        data-on:click="alert('Edit feature coming soon!')" {
                                        "Edit"
                                    }
                                    button
                                        class="editor-btn editor-btn--danger"
                                        data-on:click=[Some(PreEscaped(format!("@delete('/editor/rewards/{}')", reward.id)))] {
                                        "Delete"
                                    }
                                }
                            }
                        }
                    }
                    @if rewards.is_empty() {
                        tr {
                            td colspan="4" class="empty-message" {
                                "No rewards yet. Add your first reward!"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Settings panel for the editor
fn editor_settings_panel(settings: &Settings) -> Markup {
    html! {
        div id="settings-panel" {
            div class="panel-header" {
                h2 { "⚙️ Settings" }
            }

            (PreEscaped(r#"<form id="settings-form" class="editor-form editor-form--settings" action="/editor/settings" method="PUT" data-on:submit__prevent="return handleSettingsForm(event)">"#))
                h3 { "Weekly Goal" }

                div class="form-group" {
                    label for="weekly-exp-goal" { "Weekly EXP Goal *" }
                    input
                        type="number"
                        id="weekly-exp-goal"
                        name="weekly_exp_goal"
                        required
                        min="0"
                        value=(settings.weekly_exp_goal)
                        placeholder="100";
                    p class="form-hint" { "The total EXP required to unlock all rewards for the week" }
                }

                div class="form-actions" {
                    button
                        type="submit"
                        class="editor-btn editor-btn--primary"
                        data-indicator="#settings-saving" {
                        span { "Save Settings" }
                        span id="settings-saving" style="display: none" { "Saving..." }
                    }
                (PreEscaped("</form>"))
            }
        }
    }
}

/// Logout button
fn logout_button() -> Markup {
    html! {
        button
            type="button"
            class="logout-btn"
            data-on:click="@post('/editor/logout')"
            data-indicator="#logout-loading" {
            span { "🚪 Exit Guild" }
            span id="logout-loading" style="display: none" { "Leaving..." }
        }
    }
}
