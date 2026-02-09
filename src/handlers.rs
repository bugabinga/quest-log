use askama_axum::Template;
use axum::extract::State;
use axum::response::Html;
use chrono::Datelike;

use super::database::Database;

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
    let quests = match db.get_quests_for_day(day_of_week).await {
        Ok(quests) => quests,
        Err(_) => Vec::new(),
    };

    // Check completion status for each quest and create display structs
    let mut quests_display = Vec::new();
    for quest in quests {
        let completed_today = match db.is_quest_completed_today(quest.id, today).await {
            Ok(completed) => completed,
            Err(_) => false,
        };

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
