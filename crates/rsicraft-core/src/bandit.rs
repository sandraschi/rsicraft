//! `bandit`: advisory per-operator statistics for a task.
//!
//! NOT YET a bandit policy: it ranks Beta posterior means with hand-set biases
//! (deterministic, no sampling) over the 4 operators, and campaigns do not use
//! it. Real Thompson sampling wired into campaigns is Phase 1 of the plan.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]

use crate::operators::SearchOperator;
use crate::store::{Registry, StoreResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperatorArm {
    pub operator: SearchOperator,
    pub pulls: u32,
    pub successes: u32,
    pub total_reward: f64,
    pub mean_reward: f64,
    pub alpha: f64,
    pub beta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BanditState {
    pub task_id: String,
    pub total_cards: usize,
    pub search_phase: String,
    pub arms: Vec<OperatorArm>,
    pub recommended_operator: SearchOperator,
}

impl Registry {
    /// Compute per-operator statistics and an advisory recommendation for a task.
    pub fn get_bandit_state(&self, task_id: &str) -> StoreResult<BanditState> {
        let cards = self.list_cards(task_id, None, None, 10_000)?;
        let total_cards = cards.len();

        let mut draft_pulls = 0u32;
        let mut draft_wins = 0u32;
        let mut draft_reward = 0.0f64;

        let mut improve_pulls = 0u32;
        let mut improve_wins = 0u32;
        let mut improve_reward = 0.0f64;

        let mut debug_pulls = 0u32;
        let mut debug_wins = 0u32;
        let mut debug_reward = 0.0f64;

        let mut crossover_pulls = 0u32;
        let mut crossover_wins = 0u32;
        let mut crossover_reward = 0.0f64;

        for c in &cards {
            let win = c.execution_status == "SUCCESS"
                && (c.delta_score > 0.0 || c.operator_used == "DRAFT");
            let reward = c.delta_score.max(0.0) + (c.composite_utility * 0.5);

            match c.operator_used.as_str() {
                "DRAFT" => {
                    draft_pulls += 1;
                    if win {
                        draft_wins += 1;
                    }
                    draft_reward += reward;
                }
                "IMPROVE" => {
                    improve_pulls += 1;
                    if win {
                        improve_wins += 1;
                    }
                    improve_reward += reward;
                }
                "DEBUG" => {
                    debug_pulls += 1;
                    if win {
                        debug_wins += 1;
                    }
                    debug_reward += reward;
                }
                "CROSSOVER" => {
                    crossover_pulls += 1;
                    if win {
                        crossover_wins += 1;
                    }
                    crossover_reward += reward;
                }
                _ => {}
            }
        }

        // Contextual phase determination:
        // 1. Bootstrapping: fewer than 3 working cards -> Draft is mandatory.
        // 2. Recovery: if last 2 cards failed -> Debug is high priority.
        // 3. Maturing / Exploitation: posterior-mean ranking (no sampling yet).
        let working_count = cards
            .iter()
            .filter(|c| c.execution_status == "SUCCESS")
            .count();
        let recent_failures = cards
            .iter()
            .take(3)
            .filter(|c| c.execution_status != "SUCCESS")
            .count();

        let (search_phase, recommended) = if working_count < 2 {
            ("Bootstrapping", SearchOperator::Draft)
        } else if recent_failures >= 2 && cards.iter().any(|c| c.execution_status != "SUCCESS") {
            ("ErrorRecovery", SearchOperator::Debug)
        } else {
            // Posterior mean of Beta(1 + wins + bias, 1 + losses): deterministic, no sampling
            let score_arm = |pulls: u32, wins: u32, op_bias: f64| -> f64 {
                let alpha = 1.0 + f64::from(wins) + op_bias;
                let beta = 1.0 + f64::from(pulls.saturating_sub(wins));
                alpha / (alpha + beta)
            };

            let s_draft = score_arm(draft_pulls, draft_wins, 0.1);
            let s_improve = score_arm(improve_pulls, improve_wins, 0.4);
            let s_debug = score_arm(debug_pulls, debug_wins, 0.1);
            let s_crossover = score_arm(
                crossover_pulls,
                crossover_wins,
                if working_count >= 2 { 0.3 } else { 0.0 },
            );

            let mut best_op = SearchOperator::Improve;
            let mut best_score = s_improve;

            if s_crossover > best_score && working_count >= 2 {
                best_op = SearchOperator::Crossover;
                best_score = s_crossover;
            }
            if s_draft > best_score {
                best_op = SearchOperator::Draft;
                best_score = s_draft;
            }
            if s_debug > best_score && recent_failures > 0 {
                best_op = SearchOperator::Debug;
            }

            ("AdaptiveBayesian", best_op)
        };

        let make_arm = |op: SearchOperator, pulls: u32, wins: u32, total_rew: f64| -> OperatorArm {
            let mean = if pulls == 0 {
                0.0
            } else {
                total_rew / f64::from(pulls)
            };
            let alpha = 1.0 + f64::from(wins);
            let beta = 1.0 + f64::from(pulls.saturating_sub(wins));
            OperatorArm {
                operator: op,
                pulls,
                successes: wins,
                total_reward: (total_rew * 100.0).round() / 100.0,
                mean_reward: (mean * 100.0).round() / 100.0,
                alpha,
                beta,
            }
        };

        let arms = vec![
            make_arm(SearchOperator::Draft, draft_pulls, draft_wins, draft_reward),
            make_arm(
                SearchOperator::Improve,
                improve_pulls,
                improve_wins,
                improve_reward,
            ),
            make_arm(SearchOperator::Debug, debug_pulls, debug_wins, debug_reward),
            make_arm(
                SearchOperator::Crossover,
                crossover_pulls,
                crossover_wins,
                crossover_reward,
            ),
        ];

        Ok(BanditState {
            task_id: task_id.to_string(),
            total_cards,
            search_phase: search_phase.to_string(),
            arms,
            recommended_operator: recommended,
        })
    }
}
