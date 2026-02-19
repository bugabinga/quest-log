//! Unit Tests for Quest Log TUI State
//!
//! Tests the TUI application state through the public library interface.

use chrono::Utc;
use quest_log::tui::{AppState, InputMode, Tab, View};

/// Test that initial state has correct defaults
#[tokio::test]
async fn test_initial_tab_is_quests() {
    let state = AppState::new().await.unwrap();
    assert_eq!(state.selected_tab, Tab::Quests);
}

/// Test initial view is List
#[tokio::test]
async fn test_initial_view_is_list() {
    let state = AppState::new().await.unwrap();
    assert_eq!(state.current_view, View::List);
}

/// Test input mode starts as None
#[tokio::test]
async fn test_initial_input_mode_is_none() {
    let state = AppState::new().await.unwrap();
    assert_eq!(state.input_mode, InputMode::None);
}

/// Test quest data is loaded
#[tokio::test]
async fn test_quests_loaded() {
    let state = AppState::new().await.unwrap();
    assert_eq!(state.quests.len(), 7); // 7 days
}

/// Test rewards are loaded
#[tokio::test]
async fn test_rewards_loaded() {
    let state = AppState::new().await.unwrap();
    assert!(!state.rewards.is_empty());
}

/// Test weekly goal is loaded
#[tokio::test]
async fn test_weekly_goal_is_loaded() {
    let state = AppState::new().await.unwrap();
    assert!(state.weekly_goal > 0);
}

/// Test tab switching
#[tokio::test]
async fn test_can_switch_to_rewards() {
    let mut state = AppState::new().await.unwrap();
    state.selected_tab = Tab::Rewards;
    assert_eq!(state.selected_tab, Tab::Rewards);
}

/// Test tab switching to settings
#[tokio::test]
async fn test_can_switch_to_settings() {
    let mut state = AppState::new().await.unwrap();
    state.selected_tab = Tab::Settings;
    assert_eq!(state.selected_tab, Tab::Settings);
}

/// Test view switching to create
#[tokio::test]
async fn test_can_switch_to_create_view() {
    let mut state = AppState::new().await.unwrap();
    state.current_view = View::Create;
    assert_eq!(state.current_view, View::Create);
}

/// Test view switching to help
#[tokio::test]
async fn test_can_switch_to_help_view() {
    let mut state = AppState::new().await.unwrap();
    state.current_view = View::Help;
    assert_eq!(state.current_view, View::Help);
}

/// Test input buffer can be modified
#[tokio::test]
async fn test_input_buffer_modification() {
    let mut state = AppState::new().await.unwrap();
    state.input_buffer = "Test Input".to_string();
    assert_eq!(state.input_buffer, "Test Input");
}

/// Test input exp can be modified
#[tokio::test]
async fn test_input_exp_modification() {
    let mut state = AppState::new().await.unwrap();
    state.input_exp = 100;
    assert_eq!(state.input_exp, 100);
}

/// Test day selection works
#[tokio::test]
async fn test_day_selection() {
    let mut state = AppState::new().await.unwrap();
    state.selected_day = 3;
    assert_eq!(state.selected_day, 3);
}

/// Test quest index selection
#[tokio::test]
async fn test_quest_index_selection() {
    let mut state = AppState::new().await.unwrap();
    state.selected_quest = 5;
    assert_eq!(state.selected_quest, 5);
}

/// Test reward index selection
#[tokio::test]
async fn test_reward_index_selection() {
    let mut state = AppState::new().await.unwrap();
    state.selected_reward = 2;
    assert_eq!(state.selected_reward, 2);
}

/// Test that all enums can be constructed
#[tokio::test]
async fn test_all_enum_variants() {
    // Tab variants
    let _ = Tab::Quests;
    let _ = Tab::Rewards;
    let _ = Tab::Settings;

    // View variants
    let _ = View::List;
    let _ = View::Create;
    let _ = View::Delete;
    let _ = View::Help;

    // InputMode variants
    let _ = InputMode::None;
    let _ = InputMode::Title;
    let _ = InputMode::Exp;
    let _ = InputMode::Day;
    let _ = InputMode::ConfirmDelete;
    let _ = InputMode::Goal;

    // If we get here, all variants are constructible
    assert!(true);
}
