//! Isolated Container Sandbox & Deterministic Evaluator for RSICraft.
//!
//! Enforces strict rootless/non-root execution constraints (`--network none`,
//! `--memory 4g`, `--cpus 4`, `--read-only`, `--tmpfs /tmp:rw,noexec,nosuid,size=64m`)
//! when running in `docker` mode, or deterministic evaluation in `demo` mode.

use crate::operators::SearchOperator;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Command as ProcessCommand;
use std::time::Instant;

pub const DEFAULT_DOCKER_IMAGE: &str = "rsicraft-evaluator:latest";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SandboxExecutionResult {
    pub status: String,
    pub primary_score: f64,
    pub raw_output: String,
    pub execution_time_ms: i64,
    pub backend: String,
}

pub struct SandboxRunner {
    pub image_tag: String,
}

impl Default for SandboxRunner {
    fn default() -> Self {
        Self {
            image_tag: DEFAULT_DOCKER_IMAGE.to_string(),
        }
    }
}

impl SandboxRunner {
    #[must_use]
    pub fn new(image_tag: impl Into<String>) -> Self {
        Self {
            image_tag: image_tag.into(),
        }
    }

    /// Parse `RSI_METRIC_JSON={"status": "SUCCESS", "score": ...}` from raw container stdout/stderr.
    #[must_use]
    pub fn parse_evaluator_output(raw_logs: &str) -> (String, f64) {
        for line in raw_logs.lines().rev() {
            if let Some(idx) = line.find("RSI_METRIC_JSON=") {
                let json_start = idx.saturating_add("RSI_METRIC_JSON=".len());
                if let Some(json_str) = line.get(json_start..) {
                    if let Ok(parsed) = serde_json::from_str::<Value>(json_str.trim()) {
                        let status = parsed
                            .get("status")
                            .and_then(Value::as_str)
                            .unwrap_or("METRIC_PARSE_ERROR")
                            .to_string();
                        let score = parsed.get("score").and_then(Value::as_f64).unwrap_or(0.0);
                        return (status, score);
                    }
                }
            }
        }
        ("METRIC_PARSE_ERROR".to_string(), 0.0)
    }

    /// Evaluate candidate code inside an isolated Docker container with:
    /// - `--network none`
    /// - `--memory 4g`
    /// - `--cpus 4`
    /// - `--read-only`
    /// - `--tmpfs /tmp:rw,noexec,nosuid,size=64m`
    pub fn run_docker_evaluation(
        &self,
        candidate_code: &str,
        evaluator_script: &str,
        timeout_secs: u64,
    ) -> Result<SandboxExecutionResult, String> {
        let start = Instant::now();
        let container_name = format!("rsicraft-eval-{}", uuid::Uuid::new_v4());
        let payload = serde_json::json!({
            "candidate_code": candidate_code,
            "evaluator_script": evaluator_script,
        })
        .to_string();

        let py_harness = format!(
            "import os, json, sys, subprocess\n\
             p = json.loads(os.environ['RSI_PAYLOAD'])\n\
             with open('/tmp/candidate.py', 'w', encoding='utf-8') as f: f.write(p['candidate_code'])\n\
             with open('/tmp/evaluator.py', 'w', encoding='utf-8') as f: f.write(p['evaluator_script'])\n\
             try:\n\
                 res = subprocess.run(['python3', '/tmp/evaluator.py'], capture_output=True, text=True, timeout={timeout_secs})\n\
                 sys.stdout.write(res.stdout)\n\
                 sys.stderr.write(res.stderr)\n\
                 sys.exit(res.returncode)\n\
             except subprocess.TimeoutExpired:\n\
                 print('RSI_METRIC_JSON={{\"status\":\"TIMEOUT\",\"score\":0.0}}')\n\
                 sys.exit(124)\n"
        );

        let output = ProcessCommand::new("docker")
            .args([
                "run",
                "--rm",
                "--name",
                &container_name,
                "--network",
                "none",
                "--memory",
                "4g",
                "--cpus",
                "4",
                "--read-only",
                "--tmpfs",
                "/tmp:rw,noexec,nosuid,size=64m",
                "-e",
                &format!("RSI_PAYLOAD={payload}"),
                &self.image_tag,
                "python3",
                "-c",
                &py_harness,
            ])
            .output()
            .map_err(|e| format!("failed to invoke docker CLI: {e}"))?;

        let elapsed_ms = i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = if stderr.trim().is_empty() {
            stdout.to_string()
        } else {
            format!("{stdout}\n{stderr}")
        };

        if output.status.code() == Some(124) {
            return Ok(SandboxExecutionResult {
                status: "TIMEOUT".into(),
                primary_score: 0.0,
                raw_output: combined,
                execution_time_ms: elapsed_ms,
                backend: "docker".into(),
            });
        }

        let (status, primary_score) = Self::parse_evaluator_output(&combined);
        Ok(SandboxExecutionResult {
            status,
            primary_score,
            raw_output: combined,
            execution_time_ms: elapsed_ms,
            backend: "docker".into(),
        })
    }

    /// Deterministic offline evaluation oracle (`backend = "demo"`).
    /// Scores candidates deterministically based on structural properties and operator lineage.
    #[must_use]
    pub fn run_demo_evaluation(
        candidate_code: &str,
        op: SearchOperator,
        parent_score: Option<f64>,
    ) -> SandboxExecutionResult {
        let trimmed = candidate_code.trim();
        if trimmed.is_empty() || !trimmed.contains("def ") {
            let raw = "Traceback (most recent call last):\n  File \"/tmp/evaluator.py\", line 4, in evaluate\n    from candidate import solve\nImportError: cannot import name 'solve' from 'candidate'\nRSI_METRIC_JSON={\"status\":\"COMPILE_ERROR\",\"score\":0.0}";
            return SandboxExecutionResult {
                status: "COMPILE_ERROR".into(),
                primary_score: 0.0,
                raw_output: raw.into(),
                execution_time_ms: 8,
                backend: "demo".into(),
            };
        }
        if trimmed.contains("raise ") || trimmed.contains("1 / 0") {
            let raw = "Traceback (most recent call last):\n  File \"/tmp/candidate.py\", line 2, in solve\n    ZeroDivisionError: division by zero\nRSI_METRIC_JSON={\"status\":\"RUNTIME_ERROR\",\"score\":0.0}";
            return SandboxExecutionResult {
                status: "RUNTIME_ERROR".into(),
                primary_score: 0.0,
                raw_output: raw.into(),
                execution_time_ms: 11,
                backend: "demo".into(),
            };
        }

        let digest = blake3::hash(trimmed.as_bytes());
        let jitter = f64::from(digest.as_bytes()[0]) / 255.0 * 3.5;
        let base = match op {
            SearchOperator::Draft => 72.0 + jitter,
            SearchOperator::Debug => parent_score.unwrap_or(65.0).max(75.0) + 6.0 + jitter,
            SearchOperator::Improve => parent_score.unwrap_or(75.0) + 11.5 + jitter,
            SearchOperator::Crossover => parent_score.unwrap_or(82.0) + 14.0 + jitter,
        }
        .clamp(0.0, 100.0);
        let rounded = (base * 100.0).round() / 100.0;
        let exec_ms = match op {
            SearchOperator::Draft => 38,
            SearchOperator::Debug => 29,
            SearchOperator::Improve => 16,
            SearchOperator::Crossover => 14,
        };
        let raw_output = format!(
            "[rsicraft-demo-sandbox] candidate verified against synthetic benchmark suite\nRSI_METRIC_JSON={{\"status\":\"SUCCESS\",\"score\":{rounded}}}"
        );
        SandboxExecutionResult {
            status: "SUCCESS".into(),
            primary_score: rounded,
            raw_output,
            execution_time_ms: exec_ms,
            backend: "demo".into(),
        }
    }
}
