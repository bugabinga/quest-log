use crate::models::{ClaimState, WeeklyRewardDisplay};
use maud::{html, PreEscaped};

pub fn weekly_rewards(week_exp: i32, rewards: &[WeeklyRewardDisplay]) -> maud::Markup {
    html! {
        details class="weekly-rewards" id="weekly-rewards" {
            summary class="rewards-summary" {
                span class="rewards-icon" { "🏆" }
                span class="rewards-title" { "Weekly Rewards" }
                span class="rewards-exp" { (week_exp) " EXP" }
            }
            div class="rewards-list" {
                @for reward in rewards {
                    div class="reward-card" id=(format!("reward-{}", reward.id)) data-reward-id=(reward.id) {
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
