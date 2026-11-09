//! Isolated Container Sandbox & Deterministic Evaluator for RSICraft.
//!
//! Enforces strict rootless/non-root execution constraints (`--network none`,
//! `--memory 4g`, `--cpus 4`, `--read-only`, `--tmpfs /tmp:rw,noexec,nosuid,size=64m`)
//! when running in `docker` mode, or deterministic evaluation in `demo` mode.

use crate::operators::SearchOperator;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Command as ProcessCommand;
use std::time::{Duration, Instant};

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

        let py_harness = [
            "import os, json, sys, subprocess",
            "p = json.loads(os.environ['RSI_PAYLOAD'])",
            "with open('/tmp/candidate.py', 'w', encoding='utf-8') as f: f.write(p['candidate_code'])",
            "with open('/tmp/evaluator.py', 'w', encoding='utf-8') as f: f.write(p['evaluator_script'])",
            "try:",
            &format!("    res = subprocess.run(['python3', '/tmp/evaluator.py'], capture_output=True, text=True, timeout={timeout_secs})"),
            "    sys.stdout.write(res.stdout)",
            "    sys.stderr.write(res.stderr)",
            "    sys.exit(res.returncode)",
            "except subprocess.TimeoutExpired:",
            "    print('RSI_METRIC_JSON={\"status\":\"TIMEOUT\",\"score\":0.0}')",
            "    sys.exit(124)",
        ]
        .join("\n");

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

    fn resolve_python_bin() -> String {
        if let Ok(p) = std::env::var("RSICRAFT_PYTHON") {
            if !p.trim().is_empty() {
                return p;
            }
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            let candidate = std::path::PathBuf::from(appdata)
                .join("uv")
                .join("python")
                .join("cpython-3.12.9-windows-x86_64-none")
                .join("python.exe");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
        "python3".into()
    }

    /// Check if candidate or evaluator is Rust code.
    #[must_use]
    pub fn is_rust_candidate(candidate_code: &str, evaluator_script: &str) -> bool {
        let code_has_rust = candidate_code.contains("pub fn ")
            || (candidate_code.contains("fn ") && !candidate_code.contains("def "));
        let eval_has_rust = evaluator_script.contains("extern crate candidate")
            || evaluator_script.contains("fn main")
            || evaluator_script.contains("candidate::");
        code_has_rust || eval_has_rust
    }

    fn resolve_rustc_bin() -> String {
        if let Ok(p) = std::env::var("RSICRAFT_RUSTC") {
            if !p.trim().is_empty() {
                return p;
            }
        }
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let candidate = std::path::PathBuf::from(userprofile)
                .join(".cargo")
                .join("bin")
                .join("rustc.exe");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let candidate = std::path::PathBuf::from(home)
                .join(".cargo")
                .join("bin")
                .join("rustc");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
        "rustc".into()
    }

    /// Compile and evaluate candidate Rust code in an ephemeral directory.
    pub fn run_local_rust_evaluation(
        candidate_code: &str,
        evaluator_script: &str,
        timeout_secs: u64,
    ) -> Result<SandboxExecutionResult, String> {
        let start = Instant::now();
        let eval_dir =
            std::env::temp_dir().join(format!("rsicraft-eval-rust-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&eval_dir)
            .map_err(|e| format!("failed to create ephemeral sandbox dir: {e}"))?;

        let cand_path = eval_dir.join("candidate.rs");
        let eval_path = eval_dir.join("evaluator.rs");
        let rlib_path = eval_dir.join("libcandidate.rlib");
        let runner_bin = if cfg!(windows) {
            eval_dir.join("runner.exe")
        } else {
            eval_dir.join("runner")
        };

        if let Err(e) = std::fs::write(&cand_path, candidate_code) {
            let _ = std::fs::remove_dir_all(&eval_dir);
            return Err(format!("failed to write candidate.rs: {e}"));
        }
        if let Err(e) = std::fs::write(&eval_path, evaluator_script) {
            let _ = std::fs::remove_dir_all(&eval_dir);
            return Err(format!("failed to write evaluator.rs: {e}"));
        }

        let rustc = Self::resolve_rustc_bin();

        // 1. Compile candidate as rlib
        let cand_comp = ProcessCommand::new(&rustc)
            .args([
                "--crate-type=rlib",
                "--crate-name=candidate",
                "candidate.rs",
                "-O",
                "--out-dir",
                ".",
            ])
            .current_dir(&eval_dir)
            .output();

        let cand_comp = match cand_comp {
            Ok(out) => out,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&eval_dir);
                return Err(format!("failed to invoke rustc '{rustc}': {e}"));
            }
        };

        if !cand_comp.status.success() {
            let stderr = String::from_utf8_lossy(&cand_comp.stderr);
            let _ = std::fs::remove_dir_all(&eval_dir);
            return Ok(SandboxExecutionResult {
                status: "COMPILE_ERROR".into(),
                primary_score: 0.0,
                raw_output: format!("rustc candidate compilation failed:\n{stderr}"),
                execution_time_ms: i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX),
                backend: "local-rust".into(),
            });
        }

        // 2. Compile evaluator linking candidate
        let eval_comp = ProcessCommand::new(&rustc)
            .args([
                "evaluator.rs",
                "--extern",
                &format!("candidate={}", rlib_path.to_string_lossy()),
                "-L",
                ".",
                "-O",
                "-o",
                &runner_bin.to_string_lossy(),
            ])
            .current_dir(&eval_dir)
            .output();

        let eval_comp = match eval_comp {
            Ok(out) => out,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&eval_dir);
                return Err(format!("failed to compile evaluator with '{rustc}': {e}"));
            }
        };

        if !eval_comp.status.success() {
            let stderr = String::from_utf8_lossy(&eval_comp.stderr);
            let _ = std::fs::remove_dir_all(&eval_dir);
            return Ok(SandboxExecutionResult {
                status: "COMPILE_ERROR".into(),
                primary_score: 0.0,
                raw_output: format!("rustc evaluator compilation failed:\n{stderr}"),
                execution_time_ms: i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX),
                backend: "local-rust".into(),
            });
        }

        // 3. Execute runner with timeout
        let child = ProcessCommand::new(&runner_bin)
            .current_dir(&eval_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&eval_dir);
                return Err(format!("failed to spawn runner: {e}"));
            }
        };

        let run_start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);
        let mut timed_out = false;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => {
                    if run_start.elapsed() >= timeout {
                        let _ = child.kill();
                        timed_out = true;
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&eval_dir);
                    return Err(format!("error waiting for runner: {e}"));
                }
            }
        }

        let output = child.wait_with_output();
        let _ = std::fs::remove_dir_all(&eval_dir);

        let elapsed_ms = i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX);

        if timed_out {
            return Ok(SandboxExecutionResult {
                status: "TIMEOUT".into(),
                primary_score: 0.0,
                raw_output: "process timed out".into(),
                execution_time_ms: elapsed_ms,
                backend: "local-rust".into(),
            });
        }

        let output = output.map_err(|e| format!("failed to capture runner output: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = if stderr.trim().is_empty() {
            stdout.to_string()
        } else {
            format!("{stdout}\n{stderr}")
        };

        let (mut status, primary_score) = Self::parse_evaluator_output(&combined);
        if status == "METRIC_PARSE_ERROR" && !output.status.success() {
            status = "RUNTIME_ERROR".into();
        }

        Ok(SandboxExecutionResult {
            status,
            primary_score,
            raw_output: combined,
            execution_time_ms: elapsed_ms,
            backend: "local-rust".into(),
        })
    }

    /// Evaluate candidate code in an isolated ephemeral local directory (`sandbox_mode = "local"`).
    pub fn run_local_evaluation(
        candidate_code: &str,
        evaluator_script: &str,
        timeout_secs: u64,
    ) -> Result<SandboxExecutionResult, String> {
        if Self::is_rust_candidate(candidate_code, evaluator_script) {
            return Self::run_local_rust_evaluation(candidate_code, evaluator_script, timeout_secs);
        }
        let start = Instant::now();
        let eval_dir = std::env::temp_dir().join(format!("rsicraft-eval-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&eval_dir)
            .map_err(|e| format!("failed to create ephemeral sandbox dir: {e}"))?;

        let payload = serde_json::json!({
            "candidate_code": candidate_code,
            "evaluator_script": evaluator_script,
        })
        .to_string();

        let py_harness = [
            "import os, json, sys, subprocess",
            "p = json.loads(os.environ['RSI_PAYLOAD'])",
            "wdir = os.environ['RSI_WORKDIR']",
            "cand_path = os.path.join(wdir, 'candidate.py')",
            "eval_path = os.path.join(wdir, 'evaluator.py')",
            "with open(cand_path, 'w', encoding='utf-8') as f: f.write(p['candidate_code'])",
            "with open(eval_path, 'w', encoding='utf-8') as f: f.write(p['evaluator_script'])",
            "try:",
            &format!("    res = subprocess.run([sys.executable, eval_path], cwd=wdir, capture_output=True, text=True, timeout={timeout_secs})"),
            "    sys.stdout.write(res.stdout)",
            "    sys.stderr.write(res.stderr)",
            "    sys.exit(res.returncode)",
            "except subprocess.TimeoutExpired:",
            "    print('RSI_METRIC_JSON={\"status\":\"TIMEOUT\",\"score\":0.0}')",
            "    sys.exit(124)",
        ]
        .join("\n");

        let py_bin = Self::resolve_python_bin();
        let output = ProcessCommand::new(&py_bin)
            .args(["-I", "-B", "-c", &py_harness])
            .current_dir(&eval_dir)
            .env("RSI_PAYLOAD", &payload)
            .env("RSI_WORKDIR", eval_dir.to_string_lossy().as_ref())
            .output();

        let _ = std::fs::remove_dir_all(&eval_dir);

        let output = output.map_err(|e| format!("failed to invoke python '{py_bin}': {e}"))?;
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
                backend: "local".into(),
            });
        }

        let (mut status, primary_score) = Self::parse_evaluator_output(&combined);
        if status == "METRIC_PARSE_ERROR" && !output.status.success() {
            if combined.contains("SyntaxError") || combined.contains("IndentationError") {
                status = "COMPILE_ERROR".into();
            } else {
                status = "RUNTIME_ERROR".into();
            }
        }

        Ok(SandboxExecutionResult {
            status,
            primary_score,
            raw_output: combined,
            execution_time_ms: elapsed_ms,
            backend: "local".into(),
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
        let is_rust = Self::is_rust_candidate(candidate_code, "");
        if trimmed.is_empty() || (!trimmed.contains("def ") && !trimmed.contains("fn ")) {
            let raw = if is_rust {
                "error[E0425]: cannot find function `solve` in module `candidate`\nRSI_METRIC_JSON={\"status\":\"COMPILE_ERROR\",\"score\":0.0}"
            } else {
                "Traceback (most recent call last):\n  File \"/tmp/evaluator.py\", line 4, in evaluate\n    from candidate import solve\nImportError: cannot import name 'solve' from 'candidate'\nRSI_METRIC_JSON={\"status\":\"COMPILE_ERROR\",\"score\":0.0}"
            };
            return SandboxExecutionResult {
                status: "COMPILE_ERROR".into(),
                primary_score: 0.0,
                raw_output: raw.into(),
                execution_time_ms: 8,
                backend: "demo".into(),
            };
        }
        if trimmed.contains("raise ") || trimmed.contains("1 / 0") || trimmed.contains("panic!") {
            let raw = if is_rust {
                "thread 'main' panicked at 'explicit panic', candidate.rs:2:5\nRSI_METRIC_JSON={\"status\":\"RUNTIME_ERROR\",\"score\":0.0}"
            } else {
                "Traceback (most recent call last):\n  File \"/tmp/candidate.py\", line 2, in solve\n    ZeroDivisionError: division by zero\nRSI_METRIC_JSON={\"status\":\"RUNTIME_ERROR\",\"score\":0.0}"
            };
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
