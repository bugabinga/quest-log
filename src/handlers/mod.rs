//! HTTP handlers for the Quest Log application

pub mod editor;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, Response, StatusCode};
use axum::response::Html;
use axum::response::IntoResponse;
use axum::response::sse::{Event, Sse};
use chrono::{Datelike, NaiveDate, Weekday};
use datastar::axum::ReadSignals;
use datastar::execute_script::ExecuteScript;
use datastar::patch_elements::PatchElements;
use datastar::patch_signals::PatchSignals;
use futures::StreamExt;
use futures::stream::{self, Stream};
use serde::Deserialize;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;

use crate::models::{ClaimState, QuestStats};
use crate::state::AppState;
use crate::time;
use crate::ui;
use crate::ui::fragments::toggle::QuestDisplay;
use tracing::instrument;

#[derive(Clone, Debug)]
pub enum ServerMessage {
    Elements(String, Option<String>),
    Signals(String, Option<String>),
}

#[derive(Debug, Copy, Clone)]
pub enum AppError {
    Database,
    NotFound,
    ValidationError,
}

impl AppError {
    fn heading(&self) -> &'static str {
        match self {
            Self::Database => "Something went wrong",
            Self::NotFound => "Gone.",
            Self::ValidationError => "Wrong Day!",
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::Database => "Something went wrong on our end. Please try again later!",
            Self::NotFound => "Like your motivation. Or your quests.",
            Self::ValidationError => "You can only complete quests on their assigned day.",
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::Database => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::ValidationError => StatusCode::BAD_REQUEST,
        }
    }

    fn title(&self) -> &'static str {
        match self {
            Self::Database => "Oops!",
            Self::NotFound => "Nothing Here",
            Self::ValidationError => "Can't Do That",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response<Body> {
        let html = ui::error::error_page(self.title(), self.heading(), self.message());
        (self.status_code(), Html(html.into_string())).into_response()
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum QuestId {
    I64(i64),
    String(String),
}

impl QuestId {
    pub fn as_i64(&self) -> i64 {
        match self {
            QuestId::I64(v) => *v,
            QuestId::String(s) => s.parse().unwrap_or(0),
        }
    }
}

#[derive(Deserialize)]
pub struct QuestsQuery {
    pub date: Option<String>,
}

#[derive(Deserialize)]
pub struct ToggleQuestRequest {
    #[serde(default)]
    pub client_id: Option<String>,
    pub quest_id: QuestId,
}

fn get_fantasy_day_name(weekday: Weekday) -> &'static str {
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

#[instrument(name = "📜 GET /", skip(state, query), fields(date = ?query.date))]
pub async fn quests(
    State(state): State<AppState>,
    Query(query): Query<QuestsQuery>,
) -> Result<impl IntoResponse, AppError> {
    tracing::debug!(date = ?query.date, "📜 GET / request received");
    quests_handler(state, query.date).await
}

#[instrument(name = "📜 GET /day/:date", skip(state), fields(date = %date_str))]
pub async fn quests_with_date(
    State(state): State<AppState>,
    Path(date_str): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    tracing::debug!(date_str = %date_str, "GET /day/:date request received");
    quests_handler(state, Some(date_str)).await
}

async fn quests_handler(
    state: AppState,
    date_str: Option<String>,
) -> Result<impl IntoResponse, AppError> {
    let db = &state.db;
    let today = time::today();

    let selected_date = match &date_str {
        Some(date_str) => match time::parse_date(date_str) {
            Some(date) => date,
            None => {
                tracing::warn!(date_str = %date_str, "⚠️  Invalid date format - rejecting");
                return Err(AppError::ValidationError);
            }
        },
        None => today,
    };

    tracing::debug!(date = %selected_date, "📜 Loading quests for day");

    // Validate within current week (Monday to Sunday)
    let (week_start, week_end) = time::get_week_bounds(today);
    if selected_date < week_start || selected_date > week_end {
        tracing::warn!(date = %selected_date, week_start = %week_start, week_end = %week_end, "⚠️ Date outside current week - rejecting");
        return Err(AppError::ValidationError);
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
            return Err(AppError::Database);
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
            return Err(AppError::Database);
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
            QuestStats {
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

#[derive(Deserialize)]
pub struct NavigatePath {
    pub date: String,
}

#[instrument(name = "✨ toggle_quest", skip(state, request), fields(quest_id = request.quest_id.as_i64()))]
pub async fn toggle_quest(
    State(state): State<AppState>,
    ReadSignals(request): ReadSignals<ToggleQuestRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let db = &state.db;
    let bcast = state.bcast.clone();
    let today = time::today();
    let quest_id = request.quest_id.as_i64();

    tracing::debug!(quest_id, "✨ Toggle quest request received");

    // Check if quest exists first
    let quest = db.get_quest_by_id(quest_id).await.map_err(|e| {
        tracing::error!(error = %e, quest_id, "💥 Database error looking up quest");
        AppError::Database
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
        return Err(AppError::ValidationError);
    }

    tracing::debug!(quest_id, title = %quest.title, "Toggling quest completion");
    db.toggle_quest_completion(quest_id, today)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, quest_id, "💥 Database error toggling quest");
            AppError::Database
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
    let mut total_exp = 0;
    let mut quests_completed = 0;

    for q in &all_quests {
        if let Ok(completed) = db.is_quest_completed_today(q.id, today).await
            && completed
        {
            total_exp += q.exp_value;
            quests_completed += 1;
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
            week_exp_max += quests.iter().map(|q| q.exp_value).sum::<i32>();
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

#[instrument(name = "🧭 navigate", skip(state, path, headers))]
pub async fn navigate(
    State(state): State<AppState>,
    Path(path): Path<NavigatePath>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let db = &state.db;
    let today = time::today();

    tracing::debug!(target_date = %path.date, "🧭 Navigate request");

    let selected_date = if path.date == "today" {
        today
    } else {
        match NaiveDate::parse_from_str(&path.date, "%Y-%m-%d") {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(target_date = %path.date, error = %e, "⚠️ Failed to parse date");
                return Err(AppError::NotFound);
            }
        }
    };

    let week_start = selected_date
        - chrono::Duration::days(i64::from(selected_date.weekday().num_days_from_monday()));
    let week_end = week_start + chrono::Duration::days(6);

    if selected_date < week_start || selected_date > week_end {
        tracing::debug!(target_date = %selected_date, week_start = %week_start, week_end = %week_end, "🧭 Navigate: date outside week bounds");
        return Err(AppError::NotFound);
    }

    let day_of_week = selected_date.weekday().num_days_from_sunday().cast_signed();
    let is_today = selected_date == today;

    let quests = db
        .get_quests_for_day(day_of_week)
        .await
        .map_err(|_| AppError::Database)?;

    let mut quests_display = Vec::new();
    for quest in quests {
        let completed_today = db
            .is_quest_completed_today(quest.id, selected_date)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, quest_id = quest.id, "⚠️ Failed to check completion status");
                false
            });
        quests_display.push(QuestDisplay::from_quest(
            quest,
            completed_today,
            selected_date,
        ));
    }

    let total_exp: i32 = quests_display
        .iter()
        .filter(|q| q.completed_today)
        .map(|q| q.exp_value)
        .sum();
    let quests_completed =
        i32::try_from(quests_display.iter().filter(|q| q.completed_today).count()).unwrap_or(0);
    let exp_today_max: i32 = quests_display.iter().map(|q| q.exp_value).sum();
    let quests_total = i32::try_from(quests_display.len()).unwrap_or(0);

    let week_exp = db
        .calculate_weekly_exp(week_start, week_end)
        .await
        .unwrap_or(0);

    let mut week_exp_max = 0i32;
    for dow in 0..7 {
        if let Ok(quests) = db.get_quests_for_day(dow).await {
            week_exp_max += quests.iter().map(|q| q.exp_value).sum::<i32>();
        }
    }

    let day_name = get_fantasy_day_name(selected_date.weekday()).to_string();
    let selected_date_formatted = time::format_date_display(selected_date);
    let weekday_num = u8::try_from(selected_date.weekday().num_days_from_monday()).unwrap_or(0);

    let can_navigate_left = selected_date > week_start;
    let can_navigate_right = selected_date < week_end;
    let prev_date = time::format_date_iso(time::prev_day(selected_date));
    let next_date = time::format_date_iso(time::next_day(selected_date));

    let class_left = if can_navigate_left {
        "nav-btn left"
    } else {
        "nav-btn left disabled"
    }
    .to_string();
    let class_right = if can_navigate_right {
        "nav-btn right"
    } else {
        "nav-btn right disabled"
    }
    .to_string();

    let _navigate_dir = headers
        .get("X-Navigate-Dir")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("none");

    let day_header_html =
        ui::fragments::day_header::day_header(&day_name, &selected_date_formatted).into_string();
    let today_btn_html = ui::fragments::today_button::today_button(is_today).into_string();
    let nav_left_html =
        ui::fragments::nav_buttons::nav_buttons(&class_left, can_navigate_left, &prev_date)
            .into_string();
    let nav_right_html =
        ui::fragments::nav_buttons::nav_buttons(&class_right, can_navigate_right, &next_date)
            .into_string();
    let quest_list_html =
        ui::fragments::quest_list::quest_list(&quests_display, is_today).into_string();

    let date_iso = time::format_date_iso(selected_date);
    let url_path = if is_today {
        "/".to_string()
    } else {
        format!("/day/{}", date_iso)
    };
    let history_script = format!(
        "window.history.pushState({{date:'{}'}}, '', '{}'); document.body.setAttribute('data-weekday', '{}');",
        date_iso, url_path, weekday_num
    );

    let signals_json = serde_json::json!({
        "expToday": total_exp,
        "expTodayMax": exp_today_max,
        "weekExp": week_exp,
        "weekExpMax": week_exp_max,
        "questsCompleted": quests_completed,
        "questsTotal": quests_total,
        "currentDay": day_of_week,
        "isToday": is_today
    });

    let combined_html = format!(
        "{}\n{}\n{}\n{}\n{}",
        day_header_html, today_btn_html, nav_left_html, nav_right_html, quest_list_html
    );

    let events: Vec<Event> = vec![
        PatchElements::new(combined_html)
            .use_view_transition(true)
            .into(),
        PatchSignals::new(signals_json.to_string()).into(),
        ExecuteScript::new(history_script).into(),
    ];

    let stream = stream::iter(events.into_iter().map(Ok));
    Ok(Sse::new(stream))
}

#[derive(Deserialize)]
pub struct ClaimRewardRequest {
    #[serde(default)]
    pub client_id: Option<String>,
    pub reward_id: i64,
}

#[instrument(name = "📡 events", skip(state))]
pub async fn events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    tracing::debug!("📡 SSE connection opened - client subscribed to updates");
    let rx = state.bcast.subscribe();

    let stream = async_stream::stream! {
        let mut rx = BroadcastStream::new(rx);
        while let Some(res) = rx.next().await {
            match res {
                Ok(ServerMessage::Elements(html, origin)) => {
                    let payload = serde_json::json!({
                        "data": html,
                        "origin": origin
                    });
                    let ev = Event::default()
                        .event("datastar-patch-elements")
                        .data(payload.to_string());
                    yield Ok(ev);
                }
                Ok(ServerMessage::Signals(json, origin)) => {
                    let payload = serde_json::json!({
                        "data": json,
                        "origin": origin
                    });
                    let ev = Event::default()
                        .event("datastar-patch-signals")
                        .data(payload.to_string());
                    yield Ok(ev);
                }
                Err(_) => {}
            }
        }
    };

    Sse::new(stream)
}

#[instrument(name = "🏆 claim_reward", skip(state, request))]
pub async fn claim_reward(
    State(state): State<AppState>,
    ReadSignals(request): ReadSignals<ClaimRewardRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let db = &state.db;
    let bcast = state.bcast.clone();
    let today = time::today();
    let reward_id = request.reward_id;

    tracing::debug!(reward_id, "🏆 Claim reward request received");

    let (week_start, week_end) = time::get_week_bounds(today);

    let is_sunday = today.weekday().num_days_from_sunday() == 0;
    if !is_sunday {
        tracing::warn!(reward_id, "Claim attempted on non-Sunday");
        return Err(AppError::ValidationError);
    }

    if today < week_start || today > week_end {
        tracing::warn!(reward_id, "Claim attempted outside current week");
        return Err(AppError::ValidationError);
    }

    let claim_result = db
        .claim_reward_for_week(reward_id, week_start)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, reward_id, "💥 Database error claiming reward");
            AppError::Database
        })?;

    if !claim_result {
        tracing::warn!(
            reward_id,
            "Reward claim failed - either insufficient EXP or already claimed"
        );
        return Err(AppError::ValidationError);
    }

    let rewards = db
        .get_weekly_reward_status(week_start, today)
        .await
        .unwrap_or_default();

    let week_exp = db
        .calculate_weekly_exp(week_start, week_end)
        .await
        .unwrap_or(0);

    let all_rewards_claimed =
        !rewards.is_empty() && rewards.iter().all(|r| r.state == ClaimState::Claimed);

    let rewards_html =
        ui::fragments::weekly_rewards::weekly_rewards(week_exp, &rewards, all_rewards_claimed)
            .into_string();

    if all_rewards_claimed {
        let _ = db.create_weekly_champion(week_start).await;
    }

    let signals_json = serde_json::json!({
        "rewardClaimed": reward_id,
        "rewards": rewards,
        "weekExp": week_exp,
        "allRewardsClaimed": all_rewards_claimed
    });

    let origin = request.client_id.clone();
    let _ = bcast.send(ServerMessage::Elements(
        rewards_html.clone(),
        origin.clone(),
    ));
    let _ = bcast.send(ServerMessage::Signals(signals_json.to_string(), origin));

    let events: Vec<Event> = vec![
        PatchElements::new(rewards_html)
            .use_view_transition(true)
            .into(),
        PatchSignals::new(signals_json.to_string()).into(),
    ];

    let stream = stream::iter(events.into_iter().map(Ok));
    Ok(Sse::new(stream))
}

#[instrument(name = "🏴‍☠️ GET /bounty", skip(state))]
pub async fn bounty(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    tracing::debug!("🏴‍☠️ GET /bounty request received");
    bounty_handler(state).await
}

async fn bounty_handler(state: AppState) -> Result<impl IntoResponse, AppError> {
    let db = &state.db;
    let today = time::today();
    let (week_start, week_end) = time::get_week_bounds(today);

    let week_exp = db
        .calculate_weekly_exp(week_start, week_end)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "⚠️ Failed to calculate weekly EXP");
            0
        });

    let rewards = db
        .get_weekly_reward_status(week_start, today)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "⚠️ Failed to get reward status");
            Default::default()
        });

    let all_rewards_claimed =
        !rewards.is_empty() && rewards.iter().all(|r| r.state == ClaimState::Claimed);

    let html = ui::bounty::bounty_page(week_exp, &rewards, all_rewards_claimed);

    Ok(Html(html.into_string()).into_response())
}

// Test-only slow endpoint used by integration tests to simulate long-running requests.
// This intentionally sleeps for a few seconds before responding.
#[instrument(name = "🐢 slow")]
pub async fn slow() -> &'static str {
    tracing::debug!("🐢 Slow endpoint called");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    "done"
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Weekday;

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
