//! Frontis-MA1 (30B/35B MoE) LLM Client for RTX 4090 `llama-server` inference,
//! plus a deterministic `Demo` synthesis oracle for offline verification.

use crate::operators::SearchOperator;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

pub const DEFAULT_LLAMA_URL: &str = "http://127.0.0.1:8081";
pub const DEFAULT_MODEL_ALIAS: &str = "frontis";

/// Completion budget per synthesis call. Reasoning models spend part of it on
/// their thought preamble, so a tight budget truncates the code block.
/// Override with the `RSICRAFT_MAX_TOKENS` environment variable.
pub const DEFAULT_MAX_TOKENS: u32 = 12288;

fn configured_max_tokens() -> u32 {
    std::env::var("RSICRAFT_MAX_TOKENS")
        .ok()
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|v| (64..=65_536).contains(v))
        .unwrap_or(DEFAULT_MAX_TOKENS)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareProfile {
    pub host_identifier: String,
    pub cpu: String,
    pub ram_gb: u32,
    pub gpu: String,
    pub vram_total_gb: f64,
    pub model_architecture: String,
    pub active_parameters_b: f64,
    pub quantization: String,
    pub kv_cache_type: String,
    pub flash_attention: bool,
    pub context_window_tokens: u32,
    pub vram_allocated_min_gb: f64,
    pub vram_allocated_max_gb: f64,
    pub vram_headroom_gb: f64,
    pub llama_server_url: String,
    pub llama_server_command: String,
}

#[must_use]
pub fn goliath_4090_profile(llama_url: &str) -> HardwareProfile {
    HardwareProfile {
        host_identifier: "Goliath".into(),
        cpu: "AMD Ryzen 9 7900X / Threadripper Architecture (24 threads)".into(),
        ram_gb: 64,
        gpu: "NVIDIA GeForce RTX 4090".into(),
        vram_total_gb: 24.0,
        model_architecture: "Frontis-MA1 30B/35B MoE".into(),
        active_parameters_b: 3.0,
        quantization: "Q4_K_M / Q5_K_M".into(),
        kv_cache_type: "q4_0".into(),
        flash_attention: true,
        context_window_tokens: 16384,
        vram_allocated_min_gb: 18.0,
        vram_allocated_max_gb: 19.5,
        vram_headroom_gb: 4.5,
        llama_server_url: llama_url.to_string(),
        llama_server_command: "llama-server --model ./models/frontis-ma1-30b-q4_k_m.gguf --alias frontis --n-gpu-layers 99 --ctx-size 16384 --port 8081 --flash-attn --cache-type-k q4_0".into(),
    }
}

/// Text from the last Python or Rust code fence in `text` onward, if any.
fn last_code_fence(text: &str) -> Option<&str> {
    ["```python", "```py\n", "```rust", "```rs\n"]
        .iter()
        .filter_map(|tag| text.rfind(tag))
        .max()
        .and_then(|idx| text.get(idx..))
}
#[derive(Debug, Clone)]
pub struct LlamaClient {
    pub base_url: String,
    pub model_alias: String,
    pub timeout_secs: u64,
}

impl Default for LlamaClient {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_LLAMA_URL.to_string(),
            model_alias: DEFAULT_MODEL_ALIAS.to_string(),
            timeout_secs: 120,
        }
    }
}

impl LlamaClient {
    #[must_use]
    pub fn new(base_url: impl Into<String>, model_alias: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model_alias: model_alias.into(),
            timeout_secs: 120,
        }
    }

    /// Query `llama-server` `/v1/chat/completions` endpoint with operator-specific
    /// sampling temperature.
    pub fn complete(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        temperature: f64,
    ) -> Result<String, String> {
        let base = self.base_url.trim_end_matches('/');
        if base.is_empty() || base.len() > 253 {
            return Err("invalid llama_url length".into());
        }
        let endpoint = format!("{base}/v1/chat/completions");
        let mut payload = json!({
            "model": self.model_alias,
            "temperature": temperature,
            "max_tokens": configured_max_tokens(),
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ]
        });
        // Optional hint for reasoning models (Ollama and other OpenAI-compatible
        // servers accept it; llama-server ignores unknown fields).
        if let Ok(effort) = std::env::var("RSICRAFT_REASONING_EFFORT") {
            if matches!(effort.trim(), "low" | "medium" | "high" | "none") {
                payload["reasoning_effort"] = json!(effort.trim());
            }
        }

        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout(Duration::from_secs(self.timeout_secs))
            .build();

        let resp = agent
            .post(&endpoint)
            .set("Content-Type", "application/json")
            .send_json(payload)
            .map_err(|e| format!("llama-server request to {endpoint} failed: {e}"))?;

        let body: Value = resp
            .into_json()
            .map_err(|e| format!("invalid JSON from llama-server: {e}"))?;

        let message = body
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("message"));
        let field = |key: &str| {
            message
                .and_then(|m| m.get(key))
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
        };
        if let Some(content) = field("content") {
            return Ok(content.to_string());
        }
        // Reasoning models can spend the whole completion budget thinking and
        // return an empty answer. Their last drafted code block is still worth
        // a sandbox run (the evaluator is the judge, not the model's prose).
        let reasoning = field("reasoning").or_else(|| field("reasoning_content"));
        if let Some(start) = reasoning.and_then(last_code_fence) {
            return Ok(start.to_string());
        }
        Err("llama-server returned no message content (the completion budget was spent on reasoning without drafting code; raise RSICRAFT_MAX_TOKENS)".into())
    }

    /// Actively probe `llama-server` `/v1/models` endpoint, returning latency in ms,
    /// reachability status, and list of available models.
    pub fn probe(&self) -> Result<Value, String> {
        let base = self.base_url.trim_end_matches('/');
        if base.is_empty() || base.len() > 253 {
            return Err("invalid llama_url length".into());
        }
        let endpoint = format!("{base}/v1/models");
        let start = std::time::Instant::now();
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_millis(1500))
            .timeout(Duration::from_secs(3))
            .build();

        match agent.get(&endpoint).call() {
            Ok(resp) => {
                #[allow(clippy::cast_precision_loss)]
                let latency_ms = start.elapsed().as_millis() as u64;
                let body: Value = match resp.into_json() {
                    Ok(v) => v,
                    Err(_) => json!({}),
                };
                let models: Vec<Value> = body
                    .get("data")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                Ok(json!({
                    "status": "reachable",
                    "base_url": self.base_url,
                    "latency_ms": latency_ms,
                    "configured_model": self.model_alias,
                    "available_models": models,
                }))
            }
            Err(e) => {
                #[allow(clippy::cast_precision_loss)]
                let latency_ms = start.elapsed().as_millis() as u64;
                Ok(json!({
                    "status": "unreachable",
                    "base_url": self.base_url,
                    "latency_ms": latency_ms,
                    "configured_model": self.model_alias,
                    "error": format!("{e}"),
                }))
            }
        }
    }

    /// Deterministic offline synthesis oracle (`provider = "demo"`).
    /// Produces realistic evolutionary progressions across `DRAFT`, `DEBUG`,
    /// `IMPROVE`, and `CROSSOVER` without requiring live GPU weights.
    #[must_use]
    pub fn demo_synthesize(op: SearchOperator, task_id: &str, step_index: u32) -> String {
        match op {
            SearchOperator::Draft => format!(
                "```python\n# RSICraft Demo DRAFT candidate for task '{task_id}' (step {step_index})\ndef solve(arr):\n    if not arr:\n        return 0.0\n    best = arr[0]\n    for item in arr:\n        if item > best:\n            best = item\n    return float(best)\n```"
            ),
            SearchOperator::Debug => format!(
                "```python\n# RSICraft Demo DEBUG patched candidate for task '{task_id}' (step {step_index})\ndef solve(arr):\n    if arr is None or len(arr) == 0:\n        return 0.0\n    cleaned = [float(x) for x in arr if x is not None]\n    return max(cleaned) if cleaned else 0.0\n```"
            ),
            SearchOperator::Improve => format!(
                "```python\n# RSICraft Demo IMPROVE optimized candidate for task '{task_id}' (step {step_index})\ndef solve(arr):\n    # Vectorized C-loop builtin max with fast empty guard\n    return float(max(arr)) if arr else 0.0\n```"
            ),
            SearchOperator::Crossover => format!(
                "```python\n# RSICraft Demo CROSSOVER hybrid candidate for task '{task_id}' (step {step_index})\ndef solve(arr):\n    # Hybrid combining defensive sanitization from Parent A with builtin speed from Parent B\n    if not arr:\n        return 0.0\n    return float(max(x for x in arr if x is not None))\n```"
            ),
        }
    }
}
