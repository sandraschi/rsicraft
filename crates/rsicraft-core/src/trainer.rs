//! `trainer`: closed-loop post-training step (Frontis ladder: meta-evolution).
//!
//! Exports the task's curated SFT traces as a chat-format JSONL file and runs
//! the LoRA harness (`sandboxes/train_lora.py`) on them.
//!
//! Honesty rules for this module:
//! - Nothing is simulated. A dry run reports what would be trained and changes
//!   no state. A real run either produces an adapter file on disk or fails.
//! - Training never changes the task's active model. A trained adapter is only
//!   a candidate; `rsi_promote_gate` decides whether it replaces the baseline.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]

use crate::store::Registry;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::time::Instant;

const TRAIN_METRIC_MARKER: &str = "RSI_TRAIN_METRIC=";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrainResult {
    pub task_id: String,
    pub dry_run: bool,
    /// True only when the harness reported success and the adapter file exists.
    pub trained: bool,
    pub sft_samples: usize,
    pub dpo_samples: usize,
    pub active_generation: u32,
    pub active_model_alias: String,
    pub candidate_generation: u32,
    pub dataset_path: Option<String>,
    pub base_model: Option<String>,
    pub adapter_path: Option<String>,
    /// Metrics exactly as reported by the harness (`RSI_TRAIN_METRIC=`).
    pub metrics: Value,
    pub duration_ms: i64,
    pub next_step: String,
}

fn safe_name(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn training_root() -> PathBuf {
    if let Ok(p) = std::env::var("RSICRAFT_TRAIN_DIR") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join("rsicraft").join("training")
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Run one post-training step on the task's curated traces.
pub fn execute_train_step(
    reg: &Registry,
    task_id: &str,
    epochs: u32,
    learning_rate: f64,
    dry_run: bool,
) -> Result<TrainResult, String> {
    let task = reg
        .get_task(task_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("task '{task_id}' not found"))?;
    let sft = reg
        .export_sft_traces(task_id, 0.0)
        .map_err(|e| e.to_string())?;
    let dpo = reg.export_dpo_traces(task_id).map_err(|e| e.to_string())?;
    if sft.is_empty() {
        return Err(format!(
            "task '{task_id}' has no SFT traces yet; run more search steps first"
        ));
    }
    let candidate_generation = task.active_model_gen.saturating_add(1);
    let mut result = TrainResult {
        task_id: task_id.to_string(),
        dry_run,
        trained: false,
        sft_samples: sft.len(),
        dpo_samples: dpo.len(),
        active_generation: task.active_model_gen,
        active_model_alias: task.active_model_alias.clone(),
        candidate_generation,
        dataset_path: None,
        base_model: None,
        adapter_path: None,
        metrics: Value::Null,
        duration_ms: 0,
        next_step: String::new(),
    };
    if dry_run {
        result.next_step = format!(
            "dry run: {} SFT samples would be trained; nothing was trained and no state changed",
            sft.len()
        );
        return Ok(result);
    }

    let base_model = std::env::var("RSICRAFT_TRAIN_BASE_MODEL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            "set RSICRAFT_TRAIN_BASE_MODEL to a Hugging Face model id or local directory in safetensors format. GGUF files served by llama-server or Ollama cannot be fine-tuned directly".to_string()
        })?;
    let script = PathBuf::from(env_or("RSICRAFT_TRAIN_SCRIPT", "sandboxes/train_lora.py"));
    if !script.exists() {
        return Err(format!(
            "training harness not found at '{}'; run from the repo root or set RSICRAFT_TRAIN_SCRIPT",
            script.display()
        ));
    }
    let python = env_or("RSICRAFT_TRAIN_PYTHON", "python");

    let out_dir = training_root().join(format!("{}-g{candidate_generation}", safe_name(task_id)));
    std::fs::create_dir_all(&out_dir)
        .map_err(|e| format!("cannot create {}: {e}", out_dir.display()))?;
    let dataset_path = out_dir.join("dataset.jsonl");
    let mut jsonl = String::new();
    for trace in &sft {
        let row = json!({
            "messages": [
                { "role": "system", "content": trace.system_prompt },
                { "role": "user", "content": trace.user_prompt },
                { "role": "assistant", "content": trace.assistant_response },
            ]
        });
        jsonl.push_str(&row.to_string());
        jsonl.push('\n');
    }
    std::fs::write(&dataset_path, jsonl)
        .map_err(|e| format!("cannot write {}: {e}", dataset_path.display()))?;
    let adapter_dir = out_dir.join("adapter");

    let start = Instant::now();
    let output = ProcessCommand::new(&python)
        .arg(&script)
        .arg("--dataset")
        .arg(&dataset_path)
        .arg("--base-model")
        .arg(&base_model)
        .arg("--output-dir")
        .arg(&adapter_dir)
        .arg("--epochs")
        .arg(epochs.to_string())
        .arg("--learning-rate")
        .arg(learning_rate.to_string())
        .output()
        .map_err(|e| format!("failed to start training harness with '{python}': {e}"))?;
    result.duration_ms = i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX);
    result.dataset_path = Some(dataset_path.to_string_lossy().to_string());
    result.base_model = Some(base_model);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let metrics = stdout
        .lines()
        .rev()
        .find_map(|line| {
            line.find(TRAIN_METRIC_MARKER)
                .and_then(|i| line.get(i + TRAIN_METRIC_MARKER.len()..))
        })
        .and_then(|raw| serde_json::from_str::<Value>(raw.trim()).ok());
    let Some(metrics) = metrics else {
        let tail: String = stderr
            .chars()
            .rev()
            .take(1500)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        return Err(format!(
            "training harness exited with {:?} and reported no metrics. stderr tail:\n{tail}",
            output.status.code()
        ));
    };
    let status = metrics.get("status").and_then(Value::as_str).unwrap_or("");
    let adapter_file = adapter_dir.join("adapter_model.safetensors");
    if status != "SUCCESS" || !adapter_file.exists() {
        return Err(format!(
            "training did not produce an adapter (status '{status}', adapter file present: {}). Harness metrics: {metrics}",
            adapter_file.exists()
        ));
    }

    result.trained = true;
    result.adapter_path = Some(adapter_dir.to_string_lossy().to_string());
    result.metrics = metrics;
    result.next_step = "serve this adapter on the LLM endpoint under an alias of your choice, then run rsi_promote_gate with that alias as candidate_alias. The active model is unchanged until the gate promotes it".to_string();
    Ok(result)
}
