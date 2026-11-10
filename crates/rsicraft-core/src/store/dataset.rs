//! `store::dataset`: Rung 2 Self-Evolution Trace Curation & Dataset Export.
//!
//! Harvests the evolutionary DAG in SQLite WAL storage to extract:
//! 1. Curated SFT trajectories (positive utility steps, bug repairs, crossover wins).
//! 2. DPO / ORPO preference pairs (chosen high-utility child vs rejected faulty/lower child).
//!
//! Formats output ready for local RTX 4090 fine-tuning (Unsloth, PEFT, Hugging Face).
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]

use crate::store::{ExperienceCard, Registry, StoreError, StoreResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SftTrace {
    pub trace_id: String,
    pub task_id: String,
    pub card_id: String,
    pub operator: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub assistant_response: String,
    pub score: f64,
    pub delta: f64,
    pub runs_count: u32,
    pub score_std: f64,
    pub model_generation: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DpoTrace {
    pub prompt: String,
    pub chosen: String,
    pub rejected: String,
    pub chosen_score: f64,
    pub rejected_score: f64,
    pub delta: f64,
    pub operator: String,
    pub chosen_card_id: String,
    pub rejected_card_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatasetSummary {
    pub task_id: String,
    pub total_cards: usize,
    pub sft_traces_count: usize,
    pub dpo_traces_count: usize,
    pub positive_deltas: usize,
    pub debug_repairs: usize,
    pub crossover_advances: usize,
    pub mean_sft_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatasetExportBundle {
    pub task_id: String,
    pub format: String,
    pub summary: DatasetSummary,
    pub sft_traces: Vec<SftTrace>,
    pub dpo_traces: Vec<DpoTrace>,
}

impl Registry {
    /// Extract high-quality SFT trajectories from the task DAG.
    ///
    /// Filters cards where:
    /// - `execution_status == SUCCESS`
    /// - AND either:
    ///   a) `delta_score >= min_delta` (positive improvement)
    ///   b) `operator_used == 'DEBUG'` and parent was an error (repair trajectory)
    ///   c) `operator_used == 'DRAFT'` and card is in the top-quartile / frontier
    pub fn export_sft_traces(&self, task_id: &str, min_delta: f64) -> StoreResult<Vec<SftTrace>> {
        let task = self
            .get_task(task_id)?
            .ok_or_else(|| StoreError::NotFound(format!("task {task_id}")))?;
        let cards = self.list_cards(task_id, None, None, 10_000)?;

        let mut traces = Vec::new();
        for card in &cards {
            if card.execution_status != "SUCCESS" {
                continue;
            }

            let mut include = false;
            let mut prompt_context = String::new();

            match card.operator_used.as_str() {
                "DRAFT" => {
                    if card.primary_score >= 70.0 || card.composite_utility >= 0.60 {
                        include = true;
                        prompt_context = format!(
                            "Task: {}\nDescription: {}\nDraft an optimal solution.",
                            task.name, task.description
                        );
                    }
                }
                "IMPROVE" => {
                    if card.delta_score >= min_delta {
                        include = true;
                        let parent_code = card
                            .parent_a_id
                            .as_deref()
                            .and_then(|pid| self.get_card(pid).ok().flatten())
                            .map(|p| p.code)
                            .unwrap_or_default();
                        prompt_context = format!(
                            "Task: {}\nImprove the following solution to achieve higher throughput/lower latency.\n\n```python\n{}\n```",
                            task.name, parent_code
                        );
                    }
                }
                "DEBUG" => {
                    let parent = card
                        .parent_a_id
                        .as_deref()
                        .and_then(|pid| self.get_card(pid).ok().flatten());
                    let parent_failed = parent
                        .as_ref()
                        .is_some_and(|p| p.execution_status != "SUCCESS");
                    if parent_failed || card.delta_score > 0.0 {
                        include = true;
                        let parent_code = parent.as_ref().map(|p| p.code.as_str()).unwrap_or("");
                        let parent_err =
                            parent.as_ref().map(|p| p.raw_output.as_str()).unwrap_or("");
                        prompt_context = format!(
                            "Task: {}\nFix the runtime/compile error in the following code.\n\nError traceback:\n{}\n\nFailing code:\n```python\n{}\n```",
                            task.name, parent_err, parent_code
                        );
                    }
                }
                "CROSSOVER" if card.delta_score >= min_delta => {
                    include = true;
                    let p_a_code = card
                        .parent_a_id
                        .as_deref()
                        .and_then(|pid| self.get_card(pid).ok().flatten())
                        .map(|p| p.code)
                        .unwrap_or_default();
                    let p_b_code = card
                        .parent_b_id
                        .as_deref()
                        .and_then(|pid| self.get_card(pid).ok().flatten())
                        .map(|p| p.code)
                        .unwrap_or_default();
                    prompt_context = format!(
                        "Task: {}\nSynthesize complementary mechanisms from Parent A and Parent B into a superior solution.\n\nParent A:\n```python\n{}\n```\n\nParent B:\n```python\n{}\n```",
                        task.name, p_a_code, p_b_code
                    );
                }
                _ => {}
            }

            if include {
                let system_prompt = format!(
                    "You are an expert AI4AI Machine Learning Engineer operating operator {}.",
                    card.operator_used
                );
                traces.push(SftTrace {
                    trace_id: format!("sft-{}", card.card_id),
                    task_id: task_id.to_string(),
                    card_id: card.card_id.clone(),
                    operator: card.operator_used.clone(),
                    system_prompt,
                    user_prompt: prompt_context,
                    assistant_response: format!("```python\n{}\n```", card.code),
                    score: card.primary_score,
                    delta: card.delta_score,
                    runs_count: card.runs_count,
                    score_std: card.score_std,
                    model_generation: card.model_generation,
                });
            }
        }

        Ok(traces)
    }

    /// Extract pairwise DPO preference pairs from sibling cards sharing a parent.
    pub fn export_dpo_traces(&self, task_id: &str) -> StoreResult<Vec<DpoTrace>> {
        let cards = self.list_cards(task_id, None, None, 10_000)?;
        let mut dpo_pairs = Vec::new();

        // Group cards by parent_a_id
        use std::collections::HashMap;
        let mut parent_groups: HashMap<String, Vec<&ExperienceCard>> = HashMap::new();
        for c in &cards {
            if let Some(ref pa) = c.parent_a_id {
                parent_groups.entry(pa.clone()).or_default().push(c);
            }
        }

        for (parent_id, siblings) in parent_groups {
            if siblings.len() < 2 {
                continue;
            }
            let parent_card = self.get_card(&parent_id)?;
            let parent_code = parent_card.map(|p| p.code).unwrap_or_default();
            let prompt = format!(
                "Given parent code:\n```python\n{}\n```\nSynthesize an improved candidate.",
                parent_code
            );

            // Compare pairs
            for i in 0..siblings.len() {
                for j in (i + 1)..siblings.len() {
                    let a = siblings[i];
                    let b = siblings[j];
                    let diff = a.primary_score - b.primary_score;
                    if diff.abs() >= 1.0 {
                        let (chosen, rejected, delta) = if diff > 0.0 {
                            (a, b, diff)
                        } else {
                            (b, a, -diff)
                        };
                        dpo_pairs.push(DpoTrace {
                            prompt: prompt.clone(),
                            chosen: format!("```python\n{}\n```", chosen.code),
                            rejected: format!("```python\n{}\n```", rejected.code),
                            chosen_score: chosen.primary_score,
                            rejected_score: rejected.primary_score,
                            delta,
                            operator: chosen.operator_used.clone(),
                            chosen_card_id: chosen.card_id.clone(),
                            rejected_card_id: rejected.card_id.clone(),
                        });
                    }
                }
            }
        }

        Ok(dpo_pairs)
    }

    /// Compute dataset curation summary statistics for a task.
    pub fn get_dataset_summary(&self, task_id: &str) -> StoreResult<DatasetSummary> {
        let cards = self.list_cards(task_id, None, None, 10_000)?;
        let sft_traces = self.export_sft_traces(task_id, 0.0)?;
        let dpo_traces = self.export_dpo_traces(task_id)?;

        let mut positive_deltas = 0;
        let mut debug_repairs = 0;
        let mut crossover_advances = 0;
        let mut total_sft_score = 0.0;

        for t in &sft_traces {
            total_sft_score += t.score;
            if t.delta > 0.0 {
                positive_deltas += 1;
            }
            if t.operator == "DEBUG" {
                debug_repairs += 1;
            }
            if t.operator == "CROSSOVER" && t.delta > 0.0 {
                crossover_advances += 1;
            }
        }

        let mean_sft_score = if sft_traces.is_empty() {
            0.0
        } else {
            total_sft_score / (sft_traces.len() as f64)
        };

        Ok(DatasetSummary {
            task_id: task_id.to_string(),
            total_cards: cards.len(),
            sft_traces_count: sft_traces.len(),
            dpo_traces_count: dpo_traces.len(),
            positive_deltas,
            debug_repairs,
            crossover_advances,
            mean_sft_score,
        })
    }
}
