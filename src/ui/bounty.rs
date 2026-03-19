use crate::models::WeeklyRewardDisplay;
use crate::ui::base::{PageData, base_page};
use crate::ui::fragments::weekly_rewards::weekly_rewards;
use maud::html;

/// Renders the bounty board page showing weekly rewards and EXP progress.
#[must_use]
pub fn bounty_page(
    week_exp: i32,
    rewards: &[WeeklyRewardDisplay],
    all_rewards_claimed: bool,
) -> maud::Markup {
    let signals = "{weeklyRewardsOpen: true}".to_string();
    let computed =
        "({weekExpPercent: () => Math.round($weekExp / Math.max($weekExpMax, 1) * 100)})"
            .to_string();

    let body_content = html! {
        div data-signals=(maud::PreEscaped(&signals)) data-computed=(maud::PreEscaped(&computed)) {}

        div class="notifications" {}
        h1 class="rainbow-text" { "🏴‍☠️ Bounty Board 🏴‍☠️" }

        p class="bounty-intro" { "Ye seekin' adventure, traveler? Check out these Weekly Bounties fer yer chances at glory!" }

        div class="bounty-content" {
            (weekly_rewards(week_exp, rewards, all_rewards_claimed))
        }

        div class="bounty-footer" {
            p { "Complete quests throughout the week to earn EXP and unlock bounties!" }
            p { "All bounties can be claimed on Sunday, the Day of Rest 🏰" }
        }
    };

    let page_data = PageData {
        title: "Bounty Board".to_string(),
        body_content,
        weekday: None,
        signals: Some(signals),
        computed: Some(computed),
        show_nav: true,
        active_route: Some("/bounty".to_string()),
        extra_scripts: None,
    };

    base_page(&page_data)
}
