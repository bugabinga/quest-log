use maud::{Markup, PreEscaped, html};

use crate::models::Quest;
use crate::time;

/// Display representation of a quest for rendering.
#[derive(Debug, Clone)]
pub struct QuestDisplay {
    /// Quest database ID.
    pub id: i64,
    /// Quest title.
    pub title: String,
    /// Quest description.
    pub description: String,
    /// Experience points value.
    pub exp_value: i32,
    /// Whether quest was completed today.
    pub completed_today: bool,
    /// Whether the quest date is in the past.
    pub is_past: bool,
    /// Whether the quest date is in the future.
    pub is_future: bool,
}

impl QuestDisplay {
    /// Creates a display representation from a quest model.
    #[must_use]
    pub fn from_quest(
        quest: Quest,
        completed_today: bool,
        selected_date: chrono::NaiveDate,
    ) -> Self {
        let today = time::today();
        Self {
            id: quest.id,
            title: quest.title,
            description: quest.description.unwrap_or_default(),
            exp_value: quest.exp_value,
            completed_today,
            is_past: selected_date < today,
            is_future: selected_date > today,
        }
    }
}

/// Renders the toggle button/display for a quest.
#[must_use]
pub fn toggle(quest: &QuestDisplay) -> Markup {
    let onclick = format!(
        "@post('/quests/toggle', {{ payload: {{ quest_id: {} }} }})",
        quest.id
    );

    let item_class = if quest.completed_today {
        "quest-item completed"
    } else if quest.is_past {
        "quest-item past"
    } else if quest.is_future {
        "quest-item future"
    } else {
        "quest-item"
    };

    html! {
        div class=(item_class)
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
                @if quest.is_past {
                    div class=(if quest.completed_today { "completion-status quest-status--past completed" } else { "completion-status quest-status--past" }) {
                        @if quest.completed_today {
                            "✅ Quest Complete"
                        } @else {
                            "💀 Quest Failed"
                        }
                    }
                } @else if quest.is_future {
                    div class=(if quest.completed_today { "completion-status quest-status--future completed" } else { "completion-status quest-status--future" }) {
                        @if quest.completed_today {
                            "✅ Quest Complete"
                        } @else {
                            "🗺️ Adventure Awaits"
                        }
                    }
                } @else {
                    button class=(if quest.completed_today { "toggle-btn completed" } else { "toggle-btn" })
                         type="button"
                         data-on:click__prevent=[Some(PreEscaped(&onclick))] {
                        @if quest.completed_today {
                            "✅ Quest Complete"
                        } @else {
                            "⚔️ Mark Complete"
                        }
                    }
                }
            }
        }
    }
}
