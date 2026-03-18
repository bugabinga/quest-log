//! Quest handlers for the Quest Log

use axum::extract::{Path, Query, State};
use axum::response::Html;
use axum::response::IntoResponse;
use axum::response::sse::{Event, Sse};
use chrono::{Datelike, Weekday};
use datastar::axum::ReadSignals;
use datastar::patch_elements::PatchElements;
use datastar::patch_signals::PatchSignals;
use futures::stream::{self, Stream};
use serde::Deserialize;
use std::convert::Infallible;

use crate::handlers::AppError;
use crate::state::AppState;
use crate::time;
use crate::ui;
use crate::ui::fragments::toggle::QuestDisplay;
use tracing::instrument;

use super::ServerMessage;

/// Get the fantasy day name for a given weekday
#[must_use]
pub fn get_fantasy_day_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "Monday: Day of the Sword 🗡️",
        Weekday::Tue => "Tuesday: Day of the Shield 🛡️",
        Weekday::Wed => "Wednesday: Day of the Wand 🪄",
        Weekday::Thu => "Thursday: Day of the Tome 📚",
        Weekday::Fri => "Friday: Day of the Arcane ✨",
        Weekday::Sat => "Saturday: Day of the Crown 👑",
        Weekday::Sun => "Sunday: Day of Rest 🏰",
    }
}

/// Quest identifier that can be either a numeric ID or string
#[derive(Deserialize)]
#[serde(untagged)]
pub enum QuestId {
    /// Numeric quest ID
    I64(i64),
    /// String-encoded quest ID
    String(String),
}

impl QuestId {
    /// Convert the quest ID to a 64-bit integer
    #[must_use]
    pub fn as_i64(&self) -> i64 {
        match self {
            QuestId::I64(v) => *v,
            QuestId::String(s) => s.parse().unwrap_or(0),
        }
    }
}

/// Query parameters for the quests endpoint
#[derive(Deserialize)]
pub struct QuestsQuery {
    /// Optional date filter in YYYY-MM-DD format
    pub date: Option<String>,
}

/// Request to toggle a quest's completion status
#[derive(Deserialize)]
pub struct ToggleQuestRequest {
    /// Client identifier for SSE targeting
    #[serde(default)]
    pub client_id: Option<String>,
    /// The quest to toggle
    pub quest_id: QuestId,
}

/// Get quests for the current day
///
/// # Errors
///
/// Returns an error if database query fails
#[instrument(name = "📜 GET /", skip(state, query), fields(date = ?query.date))]
pub async fn quests(
    State(state): State<AppState>,
    Query(query): Query<QuestsQuery>,
) -> Result<impl IntoResponse, AppError> {
    tracing::debug!(date = ?query.date, "📜 GET / request received");
    quests_handler(state, query.date).await
}

#[instrument(name = "📜 GET /day/:date", skip(state), fields(date = %date_str))]
/// Get quests for a specific day
///
/// # Errors
///
/// Returns an error if database query fails
pub async fn quests_with_date(
    State(state): State<AppState>,
    Path(date_str): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    tracing::debug!(date_str = %date_str, "GET /day/:date request received");
    quests_handler(state, Some(date_str)).await
}

/// Internal handler for loading quests
///
/// # Errors
///
/// Returns an error if database query fails
pub async fn quests_handler(
    state: AppState,
    date_str: Option<String>,
) -> Result<impl IntoResponse, AppError> {
    let db = &state.db;
    let today = time::today();

    let selected_date = match &date_str {
        Some(date_str) => {
            if let Some(date) = time::parse_date(date_str) {
                date
            } else {
                tracing::warn!(date_str = %date_str, "⚠️  Invalid date format - rejecting");
                return Err(AppError::ValidationError(
                    "Quest not available today - wrong day".to_string(),
                ));
            }
        }
        None => today,
    };

    tracing::debug!(date = %selected_date, "📜 Loading quests for day");

    let (week_start, week_end) = time::get_week_bounds(today);
    if selected_date < week_start || selected_date > week_end {
        tracing::warn!(date = %selected_date, week_start = %week_start, week_end = %week_end, "⚠️ Date outside current week - rejecting");
        return Err(AppError::ValidationError(
            "Date outside current week".to_string(),
        ));
    }

    let day_of_week = selected_date.weekday().num_days_from_sunday().cast_signed();

    let quests = match db.get_quests_for_day(day_of_week).await {
        Ok(quests) => {
            tracing::debug!(
                quest_count = quests.len(),
                day = day_of_week,
                "📋 Loaded quests"
            );
            quests
        }
        Err(e) => {
            tracing::error!(error = %e, "💥 Failed to load quests from database");
            return Err(AppError::Database(e));
        }
    };

    let quest_ids: Vec<i64> = quests.iter().map(|q| q.id).collect();
    let completion_status = match db
        .get_quests_completion_status(&quest_ids, selected_date)
        .await
    {
        Ok(status) => status,
        Err(e) => {
            tracing::error!(error = %e, "💥 Failed to load completion status");
            return Err(AppError::Database(e));
        }
    };

    let quests_display: Vec<QuestDisplay> = quests
        .into_iter()
        .map(|quest| {
            let completed_today = *completion_status.get(&quest.id).unwrap_or(&false);
            QuestDisplay::from_quest(quest, completed_today, selected_date)
        })
        .collect();

    let total_exp: i32 = quests_display
        .iter()
        .filter_map(|q| q.completed_today.then_some(q.exp_value))
        .sum();

    let error_message = String::new();

    let is_today = selected_date == today;
    let day_name = get_fantasy_day_name(selected_date.weekday());
    let weekday_num = u8::try_from(selected_date.weekday().num_days_from_monday()).unwrap_or(0);
    let selected_date_formatted = time::format_date_display(selected_date);
    let can_navigate_left = selected_date > week_start;
    let can_navigate_right = selected_date < week_end;
    let prev_date = time::format_date_iso(time::prev_day(selected_date));
    let next_date = time::format_date_iso(time::next_day(selected_date));
    let class_left = if can_navigate_left {
        "nav-btn left".to_string()
    } else {
        "nav-btn left disabled".to_string()
    };
    let class_right = if can_navigate_right {
        "nav-btn right".to_string()
    } else {
        "nav-btn right disabled".to_string()
    };

    let weekly_stats = db
        .get_week_stats(today, week_start, week_end)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "⚠️ Failed to get week stats, using defaults");
            crate::models::QuestStats {
                exp_today: total_exp,
                exp_today_max: quests_display.iter().map(|q| q.exp_value).sum(),
                week_exp: 0,
                week_exp_max: 0,
                quests_completed: i32::try_from(
                    quests_display.iter().filter(|q| q.completed_today).count(),
                )
                .unwrap_or(0),
                quests_total: i32::try_from(quests_display.len()).unwrap_or(0),
            }
        });

    let html = ui::quests::quests_page(
        &quests_display,
        &error_message,
        &selected_date_formatted,
        day_name,
        is_today,
        &class_left,
        &class_right,
        can_navigate_left,
        can_navigate_right,
        &prev_date,
        &next_date,
        weekday_num,
        &weekly_stats,
    );

    Ok(Html(html.into_string()).into_response())
}

#[instrument(name = "✨ toggle_quest", skip(state, request), fields(quest_id = request.quest_id.as_i64()))]
/// Toggle quest completion status
///
/// # Errors
///
/// Returns an error if database operation fails
pub async fn toggle_quest(
    State(state): State<AppState>,
    ReadSignals(request): ReadSignals<ToggleQuestRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let db = &state.db;
    let bcast = state.bcast.clone();
    let today = time::today();
    let quest_id = request.quest_id.as_i64();

    tracing::debug!(quest_id, "✨ Toggle quest request received");

    let quest = db.get_quest_by_id(quest_id).await.map_err(|e| {
        tracing::error!(error = %e, quest_id, "💥 Database error looking up quest");
        AppError::Database(e)
    })?;

    let Some(quest) = quest else {
        tracing::warn!(quest_id, "❌ Quest not found in database");
        return Err(AppError::NotFound);
    };

    let quest_day = quest.day_of_week.cast_unsigned();
    let today_day = today.weekday().num_days_from_sunday();
    if quest_day != today_day {
        tracing::warn!(
            quest_id,
            quest_day,
            today_day,
            "⚠️  Quest not available today - wrong day"
        );
        return Err(AppError::ValidationError(
            "Quest not available today - wrong day".to_string(),
        ));
    }

    tracing::debug!(quest_id, title = %quest.title, "Toggling quest completion");
    db.toggle_quest_completion(quest_id, today)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, quest_id, "💥 Database error toggling quest");
            AppError::Database(e)
        })?;

    let completed_today = db
        .is_quest_completed_today(quest_id, today)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, quest_id, "⚠️  Failed to check completion status, defaulting to false");
            false
        });

    let quest_display = QuestDisplay::from_quest(quest, completed_today, today);

    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();
    let all_quests = db.get_quests_for_day(day_of_week).await.unwrap_or_default();
    let mut total_exp: i32 = 0;
    let mut quests_completed: i32 = 0;

    for q in &all_quests {
        if let Ok(completed) = db.is_quest_completed_today(q.id, today).await
            && completed
        {
            total_exp = total_exp.saturating_add(q.exp_value);
            quests_completed = quests_completed.saturating_add(1);
        }
    }

    let exp_today_max: i32 = all_quests.iter().map(|q| q.exp_value).sum();
    let quests_total = i32::try_from(all_quests.len()).unwrap_or(0);

    let (week_start, week_end) = time::get_week_bounds(today);
    let week_exp = db
        .calculate_weekly_exp(week_start, week_end)
        .await
        .unwrap_or(0);

    let mut week_exp_max = 0i32;
    for dow in 0..7 {
        if let Ok(quests) = db.get_quests_for_day(dow).await {
            week_exp_max =
                week_exp_max.saturating_add(quests.iter().map(|q| q.exp_value).sum::<i32>());
        }
    }

    let quest_html = ui::fragments::toggle::toggle(&quest_display).into_string();

    let signals_json = serde_json::json!({
        "expToday": total_exp,
        "expTodayMax": exp_today_max,
        "weekExp": week_exp,
        "weekExpMax": week_exp_max,
        "questsCompleted": quests_completed,
        "questsTotal": quests_total,
        "currentDay": day_of_week,
        "isToday": true
    });

    let origin = request.client_id.clone();
    if let Err(e) = bcast.send(ServerMessage::Elements(quest_html.clone(), origin.clone())) {
        tracing::error!(error = %e, "💥 Failed to broadcast elements");
    }
    if let Err(e) = bcast.send(ServerMessage::Signals(signals_json.to_string(), origin)) {
        tracing::error!(error = %e, "💥 Failed to broadcast signals");
    }
    tracing::trace!("📢 Broadcast sent");

    let quest_patch = PatchElements::new(quest_html).use_view_transition(true);
    let signals_patch = PatchSignals::new(signals_json.to_string());

    let events: Vec<Event> = vec![quest_patch.into(), signals_patch.into()];
    let stream = stream::iter(events.into_iter().map(Ok));
    Ok(Sse::new(stream))
}

/// Test endpoint that delays response for testing long-running requests
#[cfg(feature = "test-utils")]
#[instrument(name = "🐢 slow")]
pub async fn slow() -> &'static str {
    tracing::debug!("🐢 Slow endpoint called");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    "done"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_fantasy_day_name_monday() {
        assert_eq!(
            get_fantasy_day_name(Weekday::Mon),
            "Monday: Day of the Sword 🗡️"
        );
    }

    #[test]
    fn test_get_fantasy_day_name_tuesday() {
        assert_eq!(
            get_fantasy_day_name(Weekday::Tue),
            "Tuesday: Day of the Shield 🛡️"
        );
    }

    #[test]
    fn test_get_fantasy_day_name_wednesday() {
        assert_eq!(
            get_fantasy_day_name(Weekday::Wed),
            "Wednesday: Day of the Wand 🪄"
        );
    }

    #[test]
    fn test_get_fantasy_day_name_thursday() {
        assert_eq!(
            get_fantasy_day_name(Weekday::Thu),
            "Thursday: Day of the Tome 📚"
        );
    }

    #[test]
    fn test_get_fantasy_day_name_friday() {
        assert_eq!(
            get_fantasy_day_name(Weekday::Fri),
            "Friday: Day of the Arcane ✨"
        );
    }

    #[test]
    fn test_get_fantasy_day_name_saturday() {
        assert_eq!(
            get_fantasy_day_name(Weekday::Sat),
            "Saturday: Day of the Crown 👑"
        );
    }

    #[test]
    fn test_get_fantasy_day_name_sunday() {
        assert_eq!(get_fantasy_day_name(Weekday::Sun), "Sunday: Day of Rest 🏰");
    }

    #[test]
    fn test_quest_id_as_i64_with_i64_variant() {
        let id = QuestId::I64(42);
        assert_eq!(id.as_i64(), 42);
    }

    #[test]
    fn test_quest_id_as_i64_with_string_variant_valid() {
        let id = QuestId::String("123".to_string());
        assert_eq!(id.as_i64(), 123);
    }

    #[test]
    fn test_quest_id_as_i64_with_string_variant_invalid() {
        let id = QuestId::String("not_a_number".to_string());
        assert_eq!(id.as_i64(), 0);
    }

    #[test]
    fn test_quest_id_as_i64_with_string_variant_negative() {
        let id = QuestId::String("-5".to_string());
        assert_eq!(id.as_i64(), -5);
    }

    #[test]
    fn test_quest_id_as_i64_with_string_variant_whitespace() {
        let id = QuestId::String("  7  ".to_string());
        assert_eq!(id.as_i64(), 0);
    }
}
