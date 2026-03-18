use super::toggle::QuestDisplay;
use maud::html;

/// Renders the list of quests for the selected day.
#[must_use]
pub fn quest_list(quests: &[QuestDisplay], _is_today: bool) -> maud::Markup {
    html! {
        @if !quests.is_empty() {
            div class="quests-list" id="quest-list" style="view-transition-name: quest-list;" {
                @for quest in quests {
                    {(super::toggle::toggle(quest))}
                }
            }
        } @else {
            div class="quests-list" id="quest-list" style="view-transition-name: quest-list;" {
                p class="no-quests" { "Your parent forgot to assign quests for this day! Time to create some adventures! 🎮" }
            }
        }
    }
}
