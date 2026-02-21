use askama::Template;
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
use futures::stream::{self, Stream};
use serde::Deserialize;
use std::convert::Infallible;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::models::{ClaimState, Quest, QuestStats, ToggleResult};
use crate::state::AppState;
use crate::time;

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
    TemplateRender,
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Database => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::ValidationError => StatusCode::BAD_REQUEST,
            Self::TemplateRender => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn title(&self) -> &'static str {
        match self {
            Self::Database => "Well, This is Awkward",
            Self::NotFound => "Nothing Here",
            Self::ValidationError => "Can't Do That",
            Self::TemplateRender => "Template Trouble",
        }
    }

    fn heading(&self) -> &'static str {
        match self {
            Self::Database => "Something broke. Congratulations.",
            Self::NotFound => "Gone.",
            Self::ValidationError => "Wrong Day!",
            Self::TemplateRender => "Something went wrong rendering the page.",
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::Database => "You didn't do this, but you probably didn't need it to work anyway.",
            Self::NotFound => "Like your motivation. Or your quests.",
            Self::ValidationError => "You can only complete quests on their assigned day.",
            Self::TemplateRender => "The page failed to generate. Try again?",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response<Body> {
        let template = ErrorTemplate::from(self);
        match template.render() {
            Ok(html) => (self.status_code(), Html(html)).into_response(),
            Err(e) => {
                tracing::error!(error = %e, "💥 Failed to render error template");
                (
                    self.status_code(),
                    Html("<html><body><h1>Error</h1></body></html>".to_string()),
                )
                    .into_response()
            }
        }
    }
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    title: &'static str,
    heading: &'static str,
    message: &'static str,
}

impl From<AppError> for ErrorTemplate {
    fn from(err: AppError) -> Self {
        Self {
            title: err.title(),
            heading: err.heading(),
            message: err.message(),
        }
    }
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
    pub quest_id: QuestId,
    #[serde(default)]
    pub client_id: Option<String>,
}

#[derive(Template)]
#[template(path = "fragments/toggle.html")]
struct QuestToggleTemplate {
    pub quest: QuestDisplay,
    pub was_just_completed: bool,
    pub was_just_uncompleted: bool,
}

#[derive(Template)]
#[template(path = "fragments/day_header.html")]
struct DayHeaderTemplate {
    pub day_name: String,
    pub selected_date: String,
}

#[derive(Template)]
#[template(path = "fragments/today_button.html")]
struct TodayButtonTemplate {
    pub is_today: bool,
}

#[derive(Template)]
#[template(path = "fragments/nav_buttons.html")]
struct NavButtonLeftTemplate {
    pub class: String,
    pub can_navigate: bool,
    pub target_date: String,
}

#[derive(Template)]
#[template(path = "fragments/nav_button_right.html")]
struct NavButtonRightTemplate {
    pub class: String,
    pub can_navigate: bool,
    pub target_date: String,
}

#[derive(Template)]
#[template(path = "fragments/quest_list.html")]
struct QuestListTemplate {
    pub quests: Vec<QuestDisplay>,
    pub is_today: bool,
}

struct QuestDisplay {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub exp_value: i32,
    pub completed_today: bool,
}

impl QuestDisplay {
    fn from_quest(quest: Quest, completed_today: bool) -> Self {
        Self {
            id: quest.id,
            title: quest.title,
            description: quest.description.unwrap_or_default(),
            exp_value: quest.exp_value,
            completed_today,
        }
    }
}

#[derive(Template)]
#[template(path = "quests.html")]
struct QuestsTemplate {
    pub quests: Vec<QuestDisplay>,
    pub rewards: Vec<crate::models::WeeklyRewardDisplay>,
    pub week_exp: i32,
    pub error_message: String,
    pub selected_date: String,
    pub day_name: String,
    pub is_today: bool,
    pub class_left: String,
    pub class_right: String,
    pub can_navigate_left: bool,
    pub can_navigate_right: bool,
    pub prev_date: String,
    pub next_date: String,
    pub weekday_num: u8,
    pub stats: QuestStats,
}

pub async fn quests(
    State(state): State<AppState>,
    Query(query): Query<QuestsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let db = &state.db;
    let today = time::today();
    let selected_date = match &query.date {
        Some(date_str) => match time::parse_date(date_str) {
            Some(date) => date,
            None => {
                tracing::warn!(date_str = %date_str, "⚠️  Invalid date format - rejecting");
                return Ok(Html("Invalid date format. Use YYYY-MM-DD".to_string()).into_response());
            }
        },
        None => today,
    };

    tracing::debug!(date = %selected_date, "📜 Loading quests for day");

    // Validate within current week (Monday to Sunday)
    let (week_start, week_end) = time::get_week_bounds(today);
    if selected_date < week_start || selected_date > week_end {
        tracing::warn!(date = %selected_date, week_start = %week_start, week_end = %week_end, "⚠️  Date outside current week - rejecting");
        return Ok(Html("Invalid date - must be within current week".to_string()).into_response());
    }

    let day_of_week = selected_date.weekday().num_days_from_sunday() as i32;

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
            QuestDisplay::from_quest(quest, completed_today)
        })
        .collect();

    let total_exp: i32 = quests_display
        .iter()
        .filter_map(|q| q.completed_today.then_some(q.exp_value))
        .sum();

    let error_message = String::new();

    let is_today = selected_date == today;
    let day_name = get_fantasy_day_name(selected_date.weekday());
    let weekday_num = selected_date.weekday().num_days_from_monday() as u8;
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

    let stats = db
        .get_week_stats(today, week_start, week_end)
        .await
        .unwrap_or(QuestStats {
            exp_today: total_exp,
            exp_today_max: quests_display.iter().map(|q| q.exp_value).sum(),
            week_exp: 0,
            week_exp_max: 0,
            quests_completed: quests_display.iter().filter(|q| q.completed_today).count() as i32,
            quests_total: quests_display.len() as i32,
        });

    let week_exp = db
        .calculate_weekly_exp(week_start, week_end)
        .await
        .unwrap_or(0);

    let rewards = db
        .get_weekly_reward_status(week_start, today)
        .await
        .unwrap_or_default();

    let template = QuestsTemplate {
        quests: quests_display,
        rewards,
        week_exp,
        error_message,
        selected_date: selected_date_formatted,
        day_name: day_name.to_string(),
        is_today,
        class_left,
        class_right,
        can_navigate_left,
        can_navigate_right,
        prev_date,
        next_date,
        weekday_num,
        stats,
    };

    match template.render() {
        Ok(html) => Ok(Html(html).into_response()),
        Err(e) => {
            tracing::error!(error = %e, "💥 Failed to render quests template");
            Err(AppError::TemplateRender)
        }
    }
}

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

    let quest_day = quest.day_of_week as u32;
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
    let toggle_result = db
        .toggle_quest_completion(quest_id, today)
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

    let was_just_completed = toggle_result == ToggleResult::NewlyCompleted;
    let was_just_uncompleted = toggle_result == ToggleResult::NewlyUncompleted;

    let quest_display = QuestDisplay::from_quest(quest, completed_today);

    let day_of_week = today.weekday().num_days_from_sunday() as i32;
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
    let quests_total = all_quests.len() as i32;

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

    let quest_template = QuestToggleTemplate {
        quest: quest_display,
        was_just_completed,
        was_just_uncompleted,
    };

    let quest_html = quest_template.render().map_err(|e| {
        tracing::error!(error = %e, "💥 Failed to render toggle template");
        AppError::TemplateRender
    })?;

    let signals_json = serde_json::json!({
        "expToday": total_exp,
        "expTodayMax": exp_today_max,
        "weekExp": week_exp,
        "weekExpMax": week_exp_max,
        "questsCompleted": quests_completed,
        "questsTotal": quests_total
    });

    let origin = request.client_id.clone();
    let _ = bcast.send(ServerMessage::Elements(quest_html.clone(), origin.clone()));
    let _ = bcast.send(ServerMessage::Signals(signals_json.to_string(), origin));
    tracing::trace!("📢 Broadcast sent to {} client(s)", 1);

    let quest_patch = PatchElements::new(quest_html).use_view_transition(true);
    let signals_patch = PatchSignals::new(signals_json.to_string());

    let events: Vec<Event> = vec![quest_patch.into(), signals_patch.into()];
    let stream = stream::iter(events.into_iter().map(Ok));
    Ok(Sse::new(stream))
}

#[derive(Deserialize)]
pub struct NavigatePath {
    pub date: String,
}

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
                tracing::warn!(target_date = %path.date, error = %e, "⚠️  Failed to parse date");
                return Err(AppError::NotFound);
            }
        }
    };

    let week_start = selected_date
        - chrono::Duration::days(selected_date.weekday().num_days_from_monday() as i64);
    let week_end = week_start + chrono::Duration::days(6);

    if selected_date < week_start || selected_date > week_end {
        return Err(AppError::NotFound);
    }

    let day_of_week = selected_date.weekday().num_days_from_sunday() as i32;
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
                tracing::warn!(error = %e, quest_id = quest.id, "⚠️  Failed to check completion status");
                false
            });
        quests_display.push(QuestDisplay::from_quest(quest, completed_today));
    }

    let total_exp: i32 = quests_display
        .iter()
        .filter(|q| q.completed_today)
        .map(|q| q.exp_value)
        .sum();
    let quests_completed = quests_display.iter().filter(|q| q.completed_today).count() as i32;
    let exp_today_max: i32 = quests_display.iter().map(|q| q.exp_value).sum();
    let quests_total = quests_display.len() as i32;

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
    let weekday_num = selected_date.weekday().num_days_from_monday() as u8;

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

    let day_header = DayHeaderTemplate {
        day_name,
        selected_date: selected_date_formatted,
    };
    let today_btn = TodayButtonTemplate { is_today };
    let nav_left = NavButtonLeftTemplate {
        class: class_left,
        can_navigate: can_navigate_left,
        target_date: prev_date,
    };
    let nav_right = NavButtonRightTemplate {
        class: class_right,
        can_navigate: can_navigate_right,
        target_date: next_date,
    };
    let quest_list = QuestListTemplate {
        quests: quests_display,
        is_today,
    };

    let day_header_html = day_header.render().map_err(|e| {
        tracing::error!(error = %e, "💥 Failed to render day_header template");
        AppError::TemplateRender
    })?;
    let today_btn_html = today_btn.render().map_err(|e| {
        tracing::error!(error = %e, "💥 Failed to render today_btn template");
        AppError::TemplateRender
    })?;
    let nav_left_html = nav_left.render().map_err(|e| {
        tracing::error!(error = %e, "💥 Failed to render nav_left template");
        AppError::TemplateRender
    })?;
    let nav_right_html = nav_right.render().map_err(|e| {
        tracing::error!(error = %e, "💥 Failed to render nav_right template");
        AppError::TemplateRender
    })?;
    let quest_list_html = quest_list.render().map_err(|e| {
        tracing::error!(error = %e, "💥 Failed to render quest_list template");
        AppError::TemplateRender
    })?;

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
        "questsTotal": quests_total
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

pub async fn events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    tracing::debug!("📡 SSE connection opened - client subscribed to updates");
    let rx = state.bcast.subscribe();
    let bcast_stream = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(ServerMessage::Elements(html, origin)) => {
            let payload = serde_json::json!({
                "data": html,
                "origin": origin
            });
            let ev = Event::default()
                .event("datastar-patch-elements")
                .data(payload.to_string());
            Some(Ok(ev))
        }
        Ok(ServerMessage::Signals(json, origin)) => {
            let payload = serde_json::json!({
                "data": json,
                "origin": origin
            });
            let ev = Event::default()
                .event("datastar-patch-signals")
                .data(payload.to_string());
            Some(Ok(ev))
        }
        Err(_) => None,
    });

    let keepalive =
        stream::once(async { Ok::<_, Infallible>(Event::default().data(": connected")) }).chain(
            stream::repeat_with(|| Ok(Event::default().data(": keepalive")))
                .throttle(std::time::Duration::from_secs(15)),
        );

    Sse::new(bcast_stream.chain(keepalive))
}

#[derive(Template)]
#[template(path = "fragments/weekly_rewards.html")]
struct WeeklyRewardsTemplate {
    pub rewards: Vec<crate::models::WeeklyRewardDisplay>,
    pub week_exp: i32,
}

#[derive(Deserialize)]
pub struct ClaimRewardRequest {
    pub reward_id: i64,
    #[serde(default)]
    pub client_id: Option<String>,
}

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

    let rewards_template = WeeklyRewardsTemplate {
        rewards: rewards.clone(),
        week_exp,
    };

    let rewards_html = rewards_template.render().map_err(|e| {
        tracing::error!(error = %e, "💥 Failed to render rewards template");
        AppError::TemplateRender
    })?;

    let signals_json = serde_json::json!({
        "rewardClaimed": reward_id,
        "rewards": rewards,
        "weekExp": week_exp
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

// Test-only slow endpoint used by integration tests to simulate long-running requests.
// This intentionally sleeps for a few seconds before responding.
pub async fn slow() -> &'static str {
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
