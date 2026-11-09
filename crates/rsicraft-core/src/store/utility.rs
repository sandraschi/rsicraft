//! Mathematical composite utility calculator for parent selection.
//!
//! Computes:
//! `U_i = lambda_s * s_norm_i + lambda_delta * delta_norm_i + lambda_nu * nu_i`

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct UtilityHyperparams {
    pub lambda_s: f64,
    pub lambda_delta: f64,
    pub lambda_nu: f64,
}

impl Default for UtilityHyperparams {
    fn default() -> Self {
        Self {
            lambda_s: 0.50,
            lambda_delta: 0.35,
            lambda_nu: 0.15,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CandidateMetrics {
    pub primary_score: f64,
    pub parent_primary_score: Option<f64>,
    pub children_count: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct UtilityOutput {
    pub composite: f64,
    pub raw_delta: f64,
    pub novelty_score: f64,
    pub s_norm: f64,
    pub delta_norm: f64,
}

pub struct UtilityCalculator;

impl UtilityCalculator {
    #[must_use]
    pub fn calculate(
        metrics: &CandidateMetrics,
        min_score: f64,
        max_score: f64,
        min_delta: f64,
        max_delta: f64,
        params: &UtilityHyperparams,
    ) -> UtilityOutput {
        let epsilon = 1e-6;

        // 1. Raw Delta Calculation
        let raw_delta = match metrics.parent_primary_score {
            Some(parent_score) => metrics.primary_score - parent_score,
            None => 0.0,
        };

        // 2. Normalized Score Calculation
        let s_norm = if (max_score - min_score).abs() < epsilon {
            0.5
        } else {
            ((metrics.primary_score - min_score) / (max_score - min_score + epsilon))
                .clamp(0.0, 1.0)
        };

        // 3. Normalized Delta Calculation
        let delta_norm = if (max_delta - min_delta).abs() < epsilon {
            0.5
        } else {
            ((raw_delta - min_delta) / (max_delta - min_delta + epsilon)).clamp(0.0, 1.0)
        };

        // 4. Novelty Bonus Score (Inverse Children Decay)
        let novelty_score = 1.0 / (1.0 + f64::from(metrics.children_count));

        // 5. Composite Utility Weighting
        let composite = (params.lambda_s * s_norm)
            + (params.lambda_delta * delta_norm)
            + (params.lambda_nu * novelty_score);

        UtilityOutput {
            composite,
            raw_delta,
            novelty_score,
            s_norm,
            delta_norm,
        }
    }
}
