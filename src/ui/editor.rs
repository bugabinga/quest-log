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
    let signals = format!(
        "({})",
        serde_json::json!({
            "_activeTab": active_tab,
            "_showQuestForm": false,
            "_editingQuestId": null,
            "_questTitle": "",
            "_questDescription": "",
            "_questExpValue": 10,
            "_questDayOfWeek": 0,
            "_questImage": [],
            "_showRewardForm": false,
            "_editingRewardId": null,
            "_rewardTitle": "",
            "_rewardDescription": "",
            "_rewardRequiredExp": 50,
            "_rewardImage": []
        })
    );

    let computed = r"({
        _questFormValid: () => ($_questTitle ?? '').trim().length > 0,
        _questFormTitle: () => $_editingQuestId ? 'Edit Quest' : 'Add New Quest',
        _questSubmitText: () => $_editingQuestId ? 'Update Quest' : 'Save Quest',
        _questImageTooLarge: () => ($_questImage ?? []).some((file) => file.size > 5 * 1024 * 1024),
        _rewardFormValid: () => ($_rewardTitle ?? '').trim().length > 0 && Number($_rewardRequiredExp ?? 0) >= 0,
        _rewardFormTitle: () => $_editingRewardId ? 'Edit Reward' : 'Add New Reward',
        _rewardSubmitText: () => $_editingRewardId ? 'Update Reward' : 'Save Reward',
        _rewardImageTooLarge: () => ($_rewardImage ?? []).some((file) => file.size > 5 * 1024 * 1024)
    })".to_string();

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
                            data-class:editor-tab--active="$_activeTab === 'quests'"
                            data-on:click="$_activeTab = 'quests'" {
                            "📜 Quests"
                        }
                        button
                            class=(if active_tab == "rewards" { "editor-tab editor-tab--active" } else { "editor-tab" })
                            data-class:editor-tab--active="$_activeTab === 'rewards'"
                            data-on:click="$_activeTab = 'rewards'" {
                            "🎁 Rewards"
                        }
                        button
                            class=(if active_tab == "settings" { "editor-tab editor-tab--active" } else { "editor-tab" })
                            data-class:editor-tab--active="$_activeTab === 'settings'"
                            data-on:click="$_activeTab = 'settings'" {
                            "⚙️ Settings"
                        }
                    }

                    div class="editor-content" {
                        // Quests Tab
                        div class="editor-panel" style=(if active_tab == "quests" { "" } else { "display: none;" }) data-show="$_activeTab === 'quests'" {
                            (editor_quests_panel(quests))
                        }

                        // Rewards Tab
                        div class="editor-panel" style=(if active_tab == "rewards" { "" } else { "display: none;" }) data-show="$_activeTab === 'rewards'" {
                            (editor_rewards_panel(rewards))
                        }

                        // Settings Tab
                        div class="editor-panel" style=(if active_tab == "settings" { "" } else { "display: none;" }) data-show="$_activeTab === 'settings'" {
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

    let page_data = PageData {
        title: "Quest Log Editor 🗝️".to_string(),
        body_content,
        weekday: None,
        signals: Some(signals),
        computed: Some(computed),
        show_nav: true,
        active_route: Some("/editor".to_string()),
        extra_scripts: None,
    };

    base_page(&page_data)
}

/// Quests panel for the editor
fn editor_quests_panel(quests: &[Quest]) -> Markup {
    html! {
        div id="quests-panel" {
            div class="panel-header" {
                h2 { "📜 Quest Management" }
                button
                    class="editor-btn editor-btn--primary"
                    data-on:click="$_showQuestForm = true; $_editingQuestId = null; $_questTitle = ''; $_questDescription = ''; $_questExpValue = 10; $_questDayOfWeek = 0; $_questImage = []" {
                    "+ Add Quest"
                }
            }

            // Add/Edit Quest Form
            div id="quest-form-container" class="editor-form-container" style="display: none;" data-show="$_showQuestForm" {
                div class="editor-form" {
                    h3 data-text="$_questFormTitle" { "Add New Quest" }

                    div class="form-row" {
                        div class="form-group" {
                            label for="quest-title" { "Title *" }
                            input
                                type="text"
                                id="quest-title"
                                data-bind:_questTitle
                                placeholder="Quest title...";
                        }
                        div class="form-group" {
                            label for="quest-exp" { "EXP Value" }
                            input
                                type="number"
                                id="quest-exp"
                                data-bind:_questExpValue
                                min="0";
                        }
                    }

                    div class="form-row" {
                        div class="form-group" {
                            label for="quest-day" { "Day of Week" }
                            select id="quest-day" data-bind:_questDayOfWeek {
                                @for (i, name) in DAY_NAMES.iter().enumerate() {
                                    option value=(i) { (name) }
                                }
                            }
                        }
                        div class="form-group" {
                            label for="quest-image" { "Image (optional, max 5MB)" }
                            input
                                type="file"
                                id="quest-image"
                                data-bind:_questImage
                                accept="image/*";
                            (PreEscaped(r#"<p class="error" data-show="$_questImageTooLarge">Image too large (max 5MB)</p>"#))
                        }
                    }

                    div class="form-group" {
                        label for="quest-description" { "Description" }
                        textarea
                            id="quest-description"
                            data-bind:_questDescription
                            rows="3"
                            placeholder="Quest description..." {}
                    }

                    div class="form-actions" {
                        button
                            type="button"
                            class="editor-btn editor-btn--secondary"
                            data-on:click="$_showQuestForm = false; $_editingQuestId = null" {
                            "Cancel"
                        }
                        button
                            type="button"
                            class="editor-btn editor-btn--primary"
                            data-on:click="$_questFormValid && ($_editingQuestId ? @put('/editor/quests/' + $_editingQuestId) : @post('/editor/quests'))"
                            data-attr:disabled="!$_questFormValid" {
                            span data-text="$_questSubmitText" { "Save Quest" }
                        }
                    }
                }
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
                                        data-on:click=[Some(PreEscaped(format!("@get('/editor/quests/{}/edit')", quest.id)))] {
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
                    data-on:click="$_showRewardForm = true; $_editingRewardId = null; $_rewardTitle = ''; $_rewardDescription = ''; $_rewardRequiredExp = 50; $_rewardImage = []" {
                    "+ Add Reward"
                }
            }

            // Add/Edit Reward Form
            div id="reward-form-container" class="editor-form-container" style="display: none;" data-show="$_showRewardForm" {
                div class="editor-form" {
                    h3 data-text="$_rewardFormTitle" { "Add New Reward" }

                    div class="form-row" {
                        div class="form-group" {
                            label for="reward-title" { "Title *" }
                            input
                                type="text"
                                id="reward-title"
                                data-bind:_rewardTitle
                                placeholder="Reward title...";
                        }
                        div class="form-group" {
                            label for="reward-exp" { "Required EXP *" }
                            input
                                type="number"
                                id="reward-exp"
                                data-bind:_rewardRequiredExp
                                min="0";
                        }
                    }

                    div class="form-group" {
                        label for="reward-image" { "Image (optional, max 5MB)" }
                        input
                            type="file"
                            id="reward-image"
                            data-bind:_rewardImage
                            accept="image/*";
                        (PreEscaped(r#"<p class="error" data-show="$_rewardImageTooLarge">Image too large (max 5MB)</p>"#))
                    }

                    div class="form-group" {
                        label for="reward-description" { "Description" }
                        textarea
                            id="reward-description"
                            data-bind:_rewardDescription
                            rows="3"
                            placeholder="Reward description..." {}
                    }

                    div class="form-actions" {
                        button
                            type="button"
                            class="editor-btn editor-btn--secondary"
                            data-on:click="$_showRewardForm = false; $_editingRewardId = null" {
                            "Cancel"
                        }
                        button
                            type="button"
                            class="editor-btn editor-btn--primary"
                            data-on:click="$_rewardFormValid && ($_editingRewardId ? @put('/editor/rewards/' + $_editingRewardId) : @post('/editor/rewards'))"
                            data-attr:disabled="!$_rewardFormValid" {
                            span data-text="$_rewardSubmitText" { "Save Reward" }
                        }
                    }
                }
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
                                        data-on:click=[Some(PreEscaped(format!("@get('/editor/rewards/{}/edit')", reward.id)))] {
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

            (PreEscaped(r#"<form id="settings-form" class="editor-form editor-form--settings" data-on:submit__prevent="@put('/editor/settings', {contentType: 'form'})">"#))
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
