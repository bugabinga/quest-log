//! Datastar vendored runtime (v1.0.3 and v1.0.4) corrupts multi-key
//! `data-computed` objects whose values are block-bodied arrows: the
//! expression rewriter injects `return (` inside the block, raising
//! `GenerateExpression` and killing the reactive loop (tabs, data-show and
//! effects stop updating). IIFE-valued computeds poison sibling computeds
//! with `ExecuteExpression` type errors. Until upstream fixes the rewriter,
//! every `data-*` expression must stay brace-free and expression-bodied.
#![allow(
    clippy::tests_outside_test_module,
    reason = "Integration tests in tests/ are only compiled during cargo test"
)]

use chrono::Utc;
use quest_log::models::{QuestStats, Settings, WeeklyRewardDisplay};
use quest_log::ui::bounty::bounty_page;
use quest_log::ui::editor::editor_page;
use quest_log::ui::error::error_page;
use quest_log::ui::fragments::toggle::QuestDisplay;
use quest_log::ui::quests::quests_page;

/// Extract the value of every `data-*="..."` attribute in the rendered HTML.
/// Maud escapes `"` inside attribute values, so the next quote always ends
/// the value.
fn data_attribute_values(html: &str) -> Vec<String> {
    let mut values = Vec::new();
    let bytes = html.as_bytes();
    let mut i = 0;
    while let Some(start) = html[i..].find("data-") {
        let attr_start = i + start;
        i = attr_start + "data-".len();
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric()
                || bytes[i] == b'-'
                || bytes[i] == b':'
                || bytes[i] == b'_'
                || bytes[i] == b'.')
        {
            i += 1;
        }
        if html[i..].starts_with("=\"") {
            let value_start = i + 2;
            let value_end = html[value_start..]
                .find('"')
                .map_or(html.len(), |offset| value_start + offset);
            values.push(html[value_start..value_end].to_string());
            i = value_end + 1;
        }
    }
    values
}

fn assert_rewriter_safe(page: &str, html: &str) {
    for value in data_attribute_values(html) {
        assert!(
            !value.contains("=> {") && !value.contains("=&gt; {"),
            "{page}: block-bodied arrow in data-* expression breaks the vendored Datastar rewriter: {value}"
        );
        assert!(
            !value.contains("=> (("),
            "{page}: IIFE in data-* expression breaks the vendored Datastar rewriter: {value}"
        );
    }
}

fn sample_quest() -> QuestDisplay {
    QuestDisplay {
        id: 1,
        title: "Slay the Laundry Dragon".to_string(),
        description: "Sort and fold".to_string(),
        exp_value: 4,
        image_url: Some("/quests/1/image".to_string()),
        completed_today: false,
        is_past: false,
        is_future: false,
    }
}

#[test]
fn every_page_uses_rewriter_safe_datastar_expressions() {
    let settings = Settings {
        id: 1,
        updated_at: Utc::now(),
        weekly_exp_goal: 100,
    };

    assert_rewriter_safe(
        "editor (authenticated)",
        &editor_page(true, "quests", &[], &[], &settings).into_string(),
    );
    assert_rewriter_safe(
        "editor (auth modal)",
        &editor_page(false, "quests", &[], &[], &settings).into_string(),
    );

    let quests = [sample_quest()];
    let stats = QuestStats {
        exp_today: 4,
        exp_today_max: 50,
        quests_completed: 1,
        quests_total: 12,
        week_exp: 8,
        week_exp_max: 50,
    };
    assert_rewriter_safe(
        "quests page",
        &quests_page(
            &quests,
            "",
            "2026-10-04",
            "Sunday: Day of Rest 🏰",
            true,
            "nav-button--disabled",
            "nav-button",
            false,
            true,
            "2026-10-03",
            "2026-10-05",
            0,
            0,
            &stats,
        )
        .into_string(),
    );

    let rewards = [WeeklyRewardDisplay {
        can_claim_today: false,
        description: Some("Screen time".to_string()),
        id: 1,
        required_exp: 50,
        state: quest_log::models::ClaimState::Locked,
        title: "Treasure".to_string(),
        weekly_exp: 8,
    }];
    assert_rewriter_safe(
        "bounty page",
        &bounty_page(8, &rewards, false).into_string(),
    );

    assert_rewriter_safe(
        "error page",
        &error_page("Nothing Here", "Gone.", "Like your motivation.").into_string(),
    );
}
