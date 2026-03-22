//! Snapshot tests for Maud UI templates.
//!
//! Purpose: Help agents predict HTML output of Maud templates.
//!
//! # How to Review Snapshots
//!
//! Run `cargo insta review` to interactively review pending snapshot changes.
//!
//! # How to Update Snapshots
//!
//! Run `cargo insta test --accept` to accept all pending snapshot changes.
//! This is typically needed when HTML structure changes intentionally.

use chrono::NaiveDate;
use quest_log::handlers::stats::HighscoreData;
use quest_log::models::ClaimState;
use quest_log::models::WeeklyRewardDisplay;
use quest_log::ui::auth::auth_modal;
use quest_log::ui::error::error_page;
use quest_log::ui::fragments::confetti::confetti;
use quest_log::ui::fragments::day_header::day_header;
use quest_log::ui::fragments::nav_buttons::nav_buttons;
use quest_log::ui::fragments::quest_list::quest_list;
use quest_log::ui::fragments::today_button::today_button;
use quest_log::ui::fragments::toggle::{QuestDisplay, toggle};
use quest_log::ui::fragments::weekly_rewards::weekly_rewards;
use quest_log::ui::highscore::highscore_page;

// ============================================================================
// Toggle Tests
// ============================================================================

#[test]
fn toggle_quest_incomplete() {
    let fixture = QuestDisplay {
        id: 1,
        title: "Defeat the Dragon".to_string(),
        description: "Venture into the volcanic mountains and defeat the ancient dragon"
            .to_string(),
        exp_value: 100,
        completed_today: false,
        is_past: false,
        is_future: false,
    };

    let html = toggle(&fixture).into_string();
    insta::assert_snapshot!(html);
}

#[test]
fn toggle_quest_completed() {
    let fixture = QuestDisplay {
        id: 2,
        title: "Complete the Tutorial".to_string(),
        description: "Learn the basics of quest management".to_string(),
        exp_value: 50,
        completed_today: true,
        is_past: false,
        is_future: false,
    };

    let html = toggle(&fixture).into_string();
    insta::assert_snapshot!(html);
}

#[test]
fn toggle_quest_past() {
    let fixture = QuestDisplay {
        id: 3,
        title: "Recover the Lost Artifact".to_string(),
        description: "Find the ancient relic in the abandoned temple".to_string(),
        exp_value: 150,
        completed_today: false,
        is_past: true,
        is_future: false,
    };

    let html = toggle(&fixture).into_string();
    insta::assert_snapshot!(html);
}

#[test]
fn toggle_quest_future() {
    let fixture = QuestDisplay {
        id: 4,
        title: "Explore the New Dungeon".to_string(),
        description: "Chart the unknown territories beyond the western ridge".to_string(),
        exp_value: 200,
        completed_today: false,
        is_past: false,
        is_future: true,
    };

    let html = toggle(&fixture).into_string();
    insta::assert_snapshot!(html);
}

// ============================================================================
// Quest List Tests
// ============================================================================

#[test]
fn quest_list_empty() {
    let fixture: &[QuestDisplay] = &[];
    let html = quest_list(fixture, false).into_string();
    insta::assert_snapshot!(html);
}

#[test]
fn quest_list_with_quests() {
    let fixture = [
        QuestDisplay {
            id: 1,
            title: "Morning Meditation".to_string(),
            description: "Begin your day with clarity".to_string(),
            exp_value: 10,
            completed_today: true,
            is_past: false,
            is_future: false,
        },
        QuestDisplay {
            id: 2,
            title: "Defeat the Dragon".to_string(),
            description: "Venture into the volcanic mountains".to_string(),
            exp_value: 100,
            completed_today: false,
            is_past: false,
            is_future: false,
        },
        QuestDisplay {
            id: 3,
            title: "Past Quest Example".to_string(),
            description: "A quest from the past".to_string(),
            exp_value: 75,
            completed_today: true,
            is_past: true,
            is_future: false,
        },
    ];
    let html = quest_list(&fixture, false).into_string();
    insta::assert_snapshot!(html);
}

// ============================================================================
// Day Header Test
// ============================================================================

#[test]
fn day_header_typical() {
    let html = day_header("Monday", "March 21, 2026").into_string();
    insta::assert_snapshot!(html);
}

// ============================================================================
// Nav Buttons Tests
// ============================================================================

#[test]
fn nav_buttons_left_can_navigate() {
    let html = nav_buttons("nav-btn left", true, "2026-03-20").into_string();
    insta::assert_snapshot!(html);
}

#[test]
fn nav_buttons_left_cannot_navigate() {
    let html = nav_buttons("nav-btn left", false, "2026-03-20").into_string();
    insta::assert_snapshot!(html);
}

#[test]
fn nav_buttons_right_can_navigate() {
    let html = nav_buttons("nav-btn right", true, "2026-03-22").into_string();
    insta::assert_snapshot!(html);
}

// ============================================================================
// Today Button Tests
// ============================================================================

#[test]
fn today_button_is_today() {
    let html = today_button(true).into_string();
    insta::assert_snapshot!(html);
}

#[test]
fn today_button_not_today() {
    let html = today_button(false).into_string();
    insta::assert_snapshot!(html);
}

// ============================================================================
// Weekly Rewards Tests
// ============================================================================

#[test]
fn weekly_rewards_empty() {
    let rewards: &[WeeklyRewardDisplay] = &[];
    let html = weekly_rewards(0, rewards, false).into_string();
    insta::assert_snapshot!(html);
}

#[test]
fn weekly_rewards_with_mixed_states() {
    let fixture = [
        WeeklyRewardDisplay {
            id: 1,
            title: "Ice Cream".to_string(),
            description: Some("A sweet treat after hard work".to_string()),
            required_exp: 100,
            weekly_exp: 50,
            state: ClaimState::Locked,
            can_claim_today: false,
        },
        WeeklyRewardDisplay {
            id: 2,
            title: "Movie Night".to_string(),
            description: Some("Pick any movie to watch".to_string()),
            required_exp: 200,
            weekly_exp: 200,
            state: ClaimState::Claimable,
            can_claim_today: true,
        },
        WeeklyRewardDisplay {
            id: 3,
            title: "Video Game Time".to_string(),
            description: Some("One hour of gaming".to_string()),
            required_exp: 300,
            weekly_exp: 300,
            state: ClaimState::Claimed,
            can_claim_today: false,
        },
    ];
    let html = weekly_rewards(300, &fixture, false).into_string();
    insta::assert_snapshot!(html);
}

#[test]
fn weekly_rewards_all_claimed() {
    let fixture = [
        WeeklyRewardDisplay {
            id: 1,
            title: "Pizza Party".to_string(),
            description: Some("Order any pizza you want".to_string()),
            required_exp: 150,
            weekly_exp: 150,
            state: ClaimState::Claimed,
            can_claim_today: false,
        },
        WeeklyRewardDisplay {
            id: 2,
            title: "Shopping Spree".to_string(),
            description: Some("$20 credit at the item shop".to_string()),
            required_exp: 300,
            weekly_exp: 300,
            state: ClaimState::Claimed,
            can_claim_today: false,
        },
    ];
    let html = weekly_rewards(450, &fixture, true).into_string();
    insta::assert_snapshot!(html);
}

// ============================================================================
// Confetti Test
// ============================================================================

#[test]
fn confetti_basic() {
    let html = confetti().into_string();
    insta::assert_snapshot!(html);
}

// ============================================================================
// Error Page Test
// ============================================================================

#[test]
fn error_page_not_found() {
    let html = error_page(
        "Not Found",
        "404 - Quest Not Found",
        "The page you seek has vanished into the void.",
    )
    .into_string();
    insta::assert_snapshot!(html);
}

// ============================================================================
// Auth Modal Test
// ============================================================================

#[test]
fn auth_modal_basic() {
    let html = auth_modal().into_string();
    insta::assert_snapshot!(html);
}

// ============================================================================
// Highscore Page Tests
// ============================================================================

#[test]
fn highscore_page_with_data() {
    let fixture = HighscoreData {
        total_exp: 15000,
        quests_completed: 42,
        rewards_claimed: 8,
        weekly_champions: 3,
        completions_by_date: vec![
            (NaiveDate::from_ymd_opt(2026, 3, 21).unwrap(), 5),
            (NaiveDate::from_ymd_opt(2026, 3, 20).unwrap(), 3),
            (NaiveDate::from_ymd_opt(2026, 3, 19).unwrap(), 4),
            (NaiveDate::from_ymd_opt(2026, 3, 18).unwrap(), 2),
        ],
    };
    let html = highscore_page(&fixture).into_string();
    insta::assert_snapshot!(html);
}

#[test]
fn highscore_page_empty() {
    let fixture = HighscoreData {
        total_exp: 0,
        quests_completed: 0,
        rewards_claimed: 0,
        weekly_champions: 0,
        completions_by_date: vec![],
    };
    let html = highscore_page(&fixture).into_string();
    insta::assert_snapshot!(html);
}
