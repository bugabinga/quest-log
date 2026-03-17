use crate::models::{ClaimState, WeeklyRewardDisplay};
use maud::{PreEscaped, html};

#[must_use]
pub fn weekly_rewards(
    week_exp: i32,
    rewards: &[WeeklyRewardDisplay],
    all_rewards_claimed: bool,
) -> maud::Markup {
    html! {
        details class="weekly-rewards" id="weekly-rewards" data-attr:open="$weeklyRewardsOpen ? 'true' : ''" {
            summary class="rewards-summary" data-on:click__prevent="$weeklyRewardsOpen = !$weeklyRewardsOpen" {
                span class="rewards-icon" {
                    @if all_rewards_claimed {
                        "👑"
                    } @else {
                        "🏆"
                    }
                }
                span class="rewards-title" { "Weekly Rewards" }
                span class="rewards-exp" { (week_exp) " EXP" }
                @if all_rewards_claimed {
                    span class="weekly-champion-badge" { "🏅 Weekly Champion" }
                }
            }
            @if all_rewards_claimed {
                div class="celebration-banner" {
                    div class="celebration-content" {
                        span class="celebration-icon" { "🏆" }
                        span class="celebration-title" { "Achievement Unlocked: Weekly Champion!" }
                        span class="celebration-subtitle" { "You claimed all rewards this week!" }
                    }
                }
            }
            div class="rewards-list" {
                @for reward in rewards {
                    div class=(if all_rewards_claimed { "reward-card celebration-pulse" } else { "reward-card" }) id=(format!("reward-{}", reward.id)) data-reward-id=(reward.id) {
                        div class="reward-header" {
                            span class="reward-title" { (reward.title.as_str()) }
                            span class="reward-exp" { (reward.required_exp) " EXP" }
                        }
                        @if let Some(desc) = &reward.description {
                            p class="reward-description" { (desc) }
                        }
                        div class="reward-progress" {
                            div class="progress-bar reward-progress-bar" {
                                div class="progress-fill" style=(format!("width: {}%", (reward.weekly_exp * 100) / reward.required_exp.max(1))) {}
                            }
                            span class="progress-text" { (reward.weekly_exp) " / " (reward.required_exp) }
                        }
                        div class="reward-actions" {
                            @match reward.state {
                                ClaimState::Locked => {
                                    button class="claim-btn locked" disabled {
                                        @if reward.can_claim_today {
                                            "Not Enough EXP"
                                        } @else {
                                            "🔒 Available Sunday"
                                        }
                                    }
                                }
                                ClaimState::Claimable => {
                                    button class="claim-btn claimable" type="button"
                                        data-on:click__prevent=[Some(PreEscaped(format!("@post('/rewards/claim', {{ payload: {{ reward_id: {} }} }})", reward.id)))] {
                                        "⚔️ CLAIM REWARD! ⚔️"
                                    }
                                }
                                ClaimState::Claimed => {
                                    button class="claim-btn claimed" disabled {
                                        "✨ CLAIMED ✨"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
