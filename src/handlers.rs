use askama_axum::Template;
use axum::Form;
use axum::extract::State;
use axum::response::Html;
use chrono::Datelike;
use serde::Deserialize;

use super::database::Database;

#[derive(Deserialize)]
pub struct ToggleQuestRequest {
    pub quest_id: i64,
}

#[derive(Template)]
#[template(
    source = r#"
<div class="quest-item{% if quest.completed_today %} completed{% endif %}" data-quest-id="{{ quest.id }}">
    <div class="quest-content">
        <h3>{{ quest.title }}</h3>
        {% if quest.description.len() > 0 %}
        <p>{{ quest.description }}</p>
        {% endif %}
        <span class="exp-value">{{ quest.exp_value }} EXP</span>
    </div>
    <div class="quest-actions">
        <button class="toggle-btn" data-completed="{{ quest.completed_today }}" data-on-click="$$post('/quests/toggle', { quest_id: {{ quest.id }} })" data-on-click-morph=".quest-item[data-quest-id='{{ quest.id }}'], .total-exp">
            {% if quest.completed_today %}
            ✅ Completed
            {% else %}
            Mark Complete
            {% endif %}
        </button>
    </div>
</div>
<p class="total-exp">Total EXP Today: {{ total_exp }}</p>
"#,
    ext = "html"
)]
struct QuestToggleTemplate {
    pub quest: QuestDisplay,
    pub total_exp: i32,
}

struct QuestDisplay {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub exp_value: i32,
    pub completed_today: bool,
}

#[derive(Template)]
#[template(path = "quests.html")]
struct QuestsTemplate {
    pub quests: Vec<QuestDisplay>,
    pub total_exp: i32,
}

pub async fn quests(State(db): State<Database>) -> Html<String> {
    // Get today's day of week (0 = Sunday, 6 = Saturday)
    let today = chrono::Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    // Fetch quests for today
    let quests = db.get_quests_for_day(day_of_week).await.unwrap_or_default();

    // Check completion status for each quest and create display structs
    let mut quests_display = Vec::new();
    for quest in quests {
        let completed_today = db
            .is_quest_completed_today(quest.id, today)
            .await
            .unwrap_or_default();

        quests_display.push(QuestDisplay {
            id: quest.id,
            title: quest.title,
            description: quest.description.unwrap_or_default(),
            exp_value: quest.exp_value,
            completed_today,
        });
    }

    // Calculate total EXP from completed quests
    let total_exp: i32 = quests_display
        .iter()
        .filter(|q| q.completed_today)
        .map(|q| q.exp_value)
        .sum();

    // Create template with owned data
    let template = QuestsTemplate {
        quests: quests_display,
        total_exp,
    };

    Html(template.render().expect("Template rendering failed"))
}

pub async fn toggle_quest(
    State(db): State<Database>,
    Form(request): Form<ToggleQuestRequest>,
) -> Html<String> {
    let today = chrono::Utc::now().date_naive();

    // Toggle the quest completion
    let _ = db.toggle_quest_completion(request.quest_id, today).await;

    // Get the updated quest data
    let quest = match db.get_quest_by_id(request.quest_id).await {
        Ok(Some(quest)) => quest,
        _ => return Html(String::new()),
    };

    let completed_today = db
        .is_quest_completed_today(request.quest_id, today)
        .await
        .unwrap_or(false);

    let quest_display = QuestDisplay {
        id: quest.id,
        title: quest.title,
        description: quest.description.unwrap_or_default(),
        exp_value: quest.exp_value,
        completed_today,
    };

    // Calculate total EXP from all completed quests today
    let day_of_week = today.weekday().num_days_from_sunday() as i32;
    let all_quests = db.get_quests_for_day(day_of_week).await.unwrap_or_default();
    let mut total_exp = 0;

    for q in all_quests {
        if db
            .is_quest_completed_today(q.id, today)
            .await
            .unwrap_or(false)
        {
            total_exp += q.exp_value;
        }
    }

    let template = QuestToggleTemplate {
        quest: quest_display,
        total_exp,
    };
    Html(template.render().expect("Template rendering failed"))
}
