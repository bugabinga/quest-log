//! Navigation handlers for the Quest Log

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::sse::{Event, Sse};
use chrono::{Datelike, NaiveDate};
use datastar::execute_script::ExecuteScript;
use datastar::patch_elements::PatchElements;
use datastar::patch_signals::PatchSignals;
use futures::Stream;
use serde::Deserialize;
use std::convert::Infallible;

use crate::extractors::Timezone;
use crate::handlers::AppError;
use crate::sse_response;
use crate::state::AppState;
use crate::time;
use crate::ui::fragments::day_header::day_header;
use crate::ui::fragments::nav_buttons::nav_buttons;
use crate::ui::fragments::quest_list::quest_list;
use crate::ui::fragments::today_button::today_button;
use tracing::instrument;

/// Path parameters for navigation
#[derive(Deserialize)]
pub struct NavigatePath {
    /// Target date in YYYY-MM-DD format
    pub date: String,
}

/// Navigate to a different date
///
/// # Errors
///
/// Returns an error if database operation fails or date is invalid
///
/// # Panics
///
/// Panics if the week start or end date overflows
#[instrument(name = "🧭 navigate", skip(state, path, headers))]
pub async fn navigate(
    State(state): State<AppState>,
    Path(path): Path<NavigatePath>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let db = &state.db;
    let tz = Timezone::from_headers(&headers);
    let today = time::today_with_timezone(tz.as_deref());

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
        .checked_sub_days(chrono::Days::new(u64::from(
            selected_date.weekday().num_days_from_monday(),
        )))
        .unwrap_or_else(|| panic!("date overflow"));
    let week_end = week_start
        .checked_add_days(chrono::Days::new(6))
        .unwrap_or_else(|| panic!("date overflow"));

    if selected_date < week_start || selected_date > week_end {
        tracing::debug!(target_date = %selected_date, week_start = %week_start, week_end = %week_end, "🧭 Navigate: date outside week bounds");
        return Err(AppError::NotFound);
    }

    let day_of_week = selected_date.weekday().num_days_from_sunday().cast_signed();
    let is_today = selected_date == today;

    let quests = db
        .get_quests_for_day(day_of_week)
        .await
        .map_err(AppError::Database)?;

    let quest_ids: Vec<i64> = quests.iter().map(|q| q.id).collect();
    let completion_status = db
        .get_quests_completion_status(&quest_ids, selected_date)
        .await
        .map_err(AppError::Database)?;

    use crate::ui::fragments::toggle::QuestDisplay;
    let quests_display: Vec<QuestDisplay> = quests
        .into_iter()
        .map(|quest| {
            let completed_today = completion_status.get(&quest.id).copied().unwrap_or(false);
            QuestDisplay::from_quest(quest, completed_today, selected_date, today)
        })
        .collect();

    let quest_stats = db
        .get_week_stats(selected_date, week_start, week_end)
        .await
        .map_err(AppError::Database)?;

    use crate::handlers::quests::get_fantasy_day_name;
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

    let day_header_html = day_header(&day_name, &selected_date_formatted).into_string();
    let today_btn_html = today_button(is_today).into_string();
    let nav_left_html = nav_buttons(&class_left, can_navigate_left, &prev_date).into_string();
    let nav_right_html = nav_buttons(&class_right, can_navigate_right, &next_date).into_string();
    let quest_list_html = quest_list(&quests_display, is_today).into_string();

    let date_iso = time::format_date_iso(selected_date);
    let url_path = if is_today {
        "/".to_string()
    } else {
        format!("/day/{date_iso}")
    };
    let history_script = format!(
        "window.history.pushState({{date:'{date_iso}'}}, '', '{url_path}'); document.body.setAttribute('data-weekday', '{weekday_num}'); document.title = '{day_name}';"
    );

    let signals_json = serde_json::json!({
        "expToday": quest_stats.exp_today,
        "expTodayMax": quest_stats.exp_today_max,
        "weekExp": quest_stats.week_exp,
        "weekExpMax": quest_stats.week_exp_max,
        "questsCompleted": quest_stats.quests_completed,
        "questsTotal": quest_stats.quests_total,
        "currentDay": day_of_week,
        "isToday": is_today
    });

    let combined_html = format!(
        "{day_header_html}\n{today_btn_html}\n{nav_left_html}\n{nav_right_html}\n{quest_list_html}"
    );

    let events: Vec<Event> = vec![
        PatchElements::new(combined_html)
            .use_view_transition(true)
            .into(),
        PatchSignals::new(signals_json.to_string()).into(),
        ExecuteScript::new(history_script).into(),
    ];

    Ok(sse_response!(events))
}
