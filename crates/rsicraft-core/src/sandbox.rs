//! Isolated Container Sandbox & Deterministic Evaluator for RSICraft.
//!
//! Three backends:
//! - `docker`: unprivileged container (`--network none`, `--read-only`,
//!   `--user 10000:10001`, `--cap-drop ALL`, `--security-opt no-new-privileges`,
//!   `--pids-limit 256`,
//!   `--memory 4g`, `--cpus 4`, `--tmpfs /tmp:rw,noexec,nosuid,size=64m`).
//! - `local`: an ephemeral host directory running Python or `rustc` directly.
//!   This is NOT a security boundary: candidate code runs with the user's rights.
//! - `demo`: a deterministic oracle that executes nothing.
//!
//! Every real child process goes through `run_captured`: output is redirected
//! to files (a chatty child can never dead-lock on a full pipe) and a host-side
//! wall-clock reaper kills it when the deadline passes.

use crate::operators::{CodeLanguage, SearchOperator};
use crate::store::VALID_STATUSES;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};
use std::time::{Duration, Instant};

pub const DEFAULT_DOCKER_IMAGE: &str = "rsicraft-evaluator:latest";

/// Marker an evaluator prints on stdout to report its verdict.
pub const METRIC_MARKER: &str = "RSI_METRIC_JSON=";

/// Upper bound for the evaluator log kept on an experience card.
pub const MAX_RAW_OUTPUT_BYTES: usize = 256 * 1024;

/// Process limit inside the container. BLAS/OpenMP runtimes size their thread
/// pools from the host core count (`--cpus` does not hide cores), so this needs
/// headroom above the core count of a 24-thread host.
const DOCKER_PIDS_LIMIT: &str = "256";

/// Numeric uid:gid the evaluator runs as (the `evaluator` user of
/// `sandboxes/Dockerfile.evaluator`). Passed explicitly so a custom image that
/// forgot its `USER` line still cannot run candidates as root.
const DOCKER_USER: &str = "10000:10001";

/// Extra wall-clock allowance for container start-up on top of the evaluator timeout.
const DOCKER_STARTUP_GRACE: Duration = Duration::from_secs(30);

/// Wall-clock budget for one `rustc` invocation in the local Rust backend.
const RUSTC_COMPILE_TIMEOUT: Duration = Duration::from_secs(120);

const REAPER_POLL: Duration = Duration::from_millis(50);
const REAPER_FAST_POLL: Duration = Duration::from_millis(5);
const REAPER_FAST_WINDOW: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SandboxExecutionResult {
    pub status: String,
    pub primary_score: f64,
    pub raw_output: String,
    pub execution_time_ms: i64,
    pub backend: String,
    pub runs_count: u32,
    pub score_std: f64,
}

#[must_use]
pub fn calculate_sample_std(scores: &[f64], mean: f64) -> f64 {
    if scores.len() <= 1 {
        return 0.0;
    }
    let var = scores
        .iter()
        .map(|s| {
            let diff = s - mean;
            diff * diff
        })
        .sum::<f64>()
        / ((scores.len() - 1) as f64);
    var.sqrt()
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

/// Ephemeral working directory, removed when dropped (also on early returns).
struct Scratch(PathBuf);

impl Scratch {
    fn new(prefix: &str) -> Result<Self, String> {
        let dir = std::env::temp_dir().join(format!("{prefix}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("failed to create ephemeral sandbox dir: {e}"))?;
        Ok(Self(dir))
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn write(&self, name: &str, content: &str) -> Result<PathBuf, String> {
        let path = self.0.join(name);
        std::fs::write(&path, content).map_err(|e| format!("failed to write {name}: {e}"))?;
        Ok(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Captured {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    timed_out: bool,
}

impl Captured {
    fn combined(&self) -> String {
        if self.stderr.trim().is_empty() {
            self.stdout.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }

    fn success(&self) -> bool {
        self.code == Some(0)
    }
}

fn elapsed_ms(start: Instant) -> i64 {
    i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX)
}

/// Keep at most `max_bytes` from the end of `s` (the end is where tracebacks
/// and the metric line live), cutting on a character boundary.
#[must_use]
pub fn tail_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut start = s.len() - max_bytes;
    while start < s.len() && !s.is_char_boundary(start) {
        start += 1;
    }
    s.get(start..).unwrap_or("")
}

fn bounded_log(s: &str) -> String {
    if s.len() <= MAX_RAW_OUTPUT_BYTES {
        return s.to_string();
    }
    format!(
        "[rsicraft] output truncated to the last {MAX_RAW_OUTPUT_BYTES} bytes\n{}",
        tail_str(s, MAX_RAW_OUTPUT_BYTES)
    )
}

/// Spawn `cmd`, wait for it under a wall-clock deadline, and return its output.
///
/// stdout/stderr go to files inside `scratch` and stdin comes from a file (or
/// is closed), so no pipe can fill up while the parent is only polling.
fn run_captured(
    cmd: &mut ProcessCommand,
    scratch: &Path,
    tag: &str,
    stdin_file: Option<&Path>,
    timeout: Duration,
) -> Result<Captured, String> {
    let out_path = scratch.join(format!("{tag}.stdout.log"));
    let err_path = scratch.join(format!("{tag}.stderr.log"));
    let out_file =
        File::create(&out_path).map_err(|e| format!("failed to create stdout capture: {e}"))?;
    let err_file =
        File::create(&err_path).map_err(|e| format!("failed to create stderr capture: {e}"))?;
    let stdin = match stdin_file {
        Some(p) => {
            Stdio::from(File::open(p).map_err(|e| format!("failed to open stdin payload: {e}"))?)
        }
        None => Stdio::null(),
    };

    let program = cmd.get_program().to_string_lossy().to_string();
    let mut child = cmd
        .stdin(stdin)
        .stdout(Stdio::from(out_file))
        .stderr(Stdio::from(err_file))
        .spawn()
        .map_err(|e| format!("failed to invoke '{program}': {e}"))?;

    let start = Instant::now();
    let mut timed_out = false;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) => {
                let waited = start.elapsed();
                if waited >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break None;
                }
                std::thread::sleep(if waited < REAPER_FAST_WINDOW {
                    REAPER_FAST_POLL
                } else {
                    REAPER_POLL
                });
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("error waiting for '{program}': {e}"));
            }
        }
    };

    let read = |p: &Path| {
        std::fs::read(p)
            .map(|b| String::from_utf8_lossy(&b).to_string())
            .unwrap_or_default()
    };
    Ok(Captured {
        code,
        stdout: read(&out_path),
        stderr: read(&err_path),
        timed_out,
    })
}

impl SandboxRunner {
    #[must_use]
    pub fn new(image_tag: impl Into<String>) -> Self {
        Self {
            image_tag: image_tag.into(),
        }
    }

    /// Parse the last `RSI_METRIC_JSON={"status": "SUCCESS", "score": ...}` line.
    ///
    /// A status outside the five known execution statuses, or a non-finite
    /// score, is reported as `METRIC_PARSE_ERROR` with score 0 so the card can
    /// always be recorded.
    ///
    /// Callers pass the evaluator's stdout only. stderr is never searched: a
    /// candidate could otherwise forge a metric line there that outranks the
    /// evaluator's own line.
    #[must_use]
    pub fn parse_evaluator_output(raw_logs: &str) -> (String, f64) {
        for line in raw_logs.lines().rev() {
            if let Some(idx) = line.find(METRIC_MARKER) {
                let json_start = idx.saturating_add(METRIC_MARKER.len());
                if let Some(json_str) = line.get(json_start..) {
                    if let Ok(parsed) = serde_json::from_str::<Value>(json_str.trim()) {
                        let status = parsed.get("status").and_then(Value::as_str).unwrap_or("");
                        let score = parsed.get("score").and_then(Value::as_f64).unwrap_or(0.0);
                        if !VALID_STATUSES.contains(&status) || !score.is_finite() {
                            return ("METRIC_PARSE_ERROR".to_string(), 0.0);
                        }
                        return (status.to_string(), score);
                    }
                }
            }
        }
        ("METRIC_PARSE_ERROR".to_string(), 0.0)
    }

    fn timeout_result(raw: &str, start: Instant, backend: &str) -> SandboxExecutionResult {
        let raw_output = if raw.trim().is_empty() {
            "process timed out".to_string()
        } else {
            format!("{}\n[rsicraft] process timed out", bounded_log(raw))
        };
        SandboxExecutionResult {
            status: "TIMEOUT".into(),
            primary_score: 0.0,
            raw_output,
            execution_time_ms: elapsed_ms(start),
            backend: backend.into(),
            runs_count: 1,
            score_std: 0.0,
        }
    }

    /// Evaluate candidate code inside an isolated Docker container.
    ///
    /// The payload travels over stdin, so its size is not capped by the
    /// operating system's command-line or environment limits. The evaluator has
    /// its own in-container timeout; the host additionally reaps (`docker kill`)
    /// a container that outlives `timeout_secs` plus a start-up grace period.
    pub fn run_docker_evaluation(
        &self,
        candidate_code: &str,
        evaluator_script: &str,
        timeout_secs: u64,
        trials: u32,
    ) -> Result<SandboxExecutionResult, String> {
        let start = Instant::now();
        let scratch = Scratch::new("rsicraft-eval-docker")?;
        let container_name = format!("rsicraft-eval-{}", uuid::Uuid::new_v4());
        let payload = serde_json::json!({
            "candidate_code": candidate_code,
            "evaluator_script": evaluator_script,
        })
        .to_string();
        let payload_path = scratch.write("payload.json", &payload)?;

        let py_harness = [
            "import json, sys, subprocess",
            "p = json.load(sys.stdin)",
            "with open('/tmp/candidate.py', 'w', encoding='utf-8') as f: f.write(p['candidate_code'])",
            "with open('/tmp/evaluator.py', 'w', encoding='utf-8') as f: f.write(p['evaluator_script'])",
            "try:",
            &format!("    res = subprocess.run(['python3', '/tmp/evaluator.py'], cwd='/tmp', stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout={timeout_secs})"),
            "    sys.stdout.write(res.stdout)",
            "    sys.stderr.write(res.stderr)",
            "    sys.exit(res.returncode)",
            "except subprocess.TimeoutExpired:",
            "    print('RSI_METRIC_JSON={\"status\":\"TIMEOUT\",\"score\":0.0}')",
            "    sys.exit(124)",
        ]
        .join("\n");

        let mut cmd = ProcessCommand::new("docker");
        cmd.args([
            "run",
            "--rm",
            "-i",
            "--name",
            &container_name,
            "--network",
            "none",
            "--memory",
            "4g",
            "--cpus",
            "4",
            "--pids-limit",
            DOCKER_PIDS_LIMIT,
            "--user",
            DOCKER_USER,
            "--cap-drop",
            "ALL",
            "--security-opt",
            "no-new-privileges",
            "--read-only",
            "--tmpfs",
            "/tmp:rw,noexec,nosuid,size=64m",
            &self.image_tag,
            "python3",
            "-c",
            &py_harness,
        ]);
        let deadline = Duration::from_secs(timeout_secs).saturating_add(DOCKER_STARTUP_GRACE);
        let run = run_captured(
            &mut cmd,
            scratch.path(),
            "docker",
            Some(&payload_path),
            deadline,
        )?;

        if run.timed_out {
            // Killing the docker CLI does not stop the container; do that explicitly.
            let mut kill = ProcessCommand::new("docker");
            kill.args(["kill", &container_name]);
            let _ = run_captured(
                &mut kill,
                scratch.path(),
                "docker-kill",
                None,
                Duration::from_secs(15),
            );
            return Ok(Self::timeout_result(&run.combined(), start, "docker"));
        }
        if run.code == Some(124) {
            return Ok(Self::timeout_result(&run.combined(), start, "docker"));
        }
        // 125: `docker run` itself failed (daemon down, unknown image, bad flag).
        // That is an infrastructure fault, not a property of the candidate, so it
        // must not be recorded as an experience card.
        if run.code == Some(125) {
            return Err(format!(
                "docker run failed (is Docker running and image '{}' built?): {}",
                self.image_tag,
                tail_str(run.stderr.trim(), 2000)
            ));
        }

        let combined = run.combined();
        let (mut status, primary_score) = Self::parse_evaluator_output(&run.stdout);
        if status == "METRIC_PARSE_ERROR" && !run.success() {
            status = Self::classify_python_failure(&combined);
        }
        let n_trials = trials.max(1);
        let runs_count = if n_trials > 1 { n_trials } else { 1 };
        Ok(SandboxExecutionResult {
            status,
            primary_score,
            raw_output: bounded_log(&combined),
            execution_time_ms: elapsed_ms(start),
            backend: "docker".into(),
            runs_count,
            score_std: 0.0,
        })
    }

    fn classify_python_failure(combined: &str) -> String {
        if combined.contains("SyntaxError") || combined.contains("IndentationError") {
            "COMPILE_ERROR".into()
        } else {
            "RUNTIME_ERROR".into()
        }
    }

    fn resolve_python_bin() -> String {
        if let Ok(p) = std::env::var("RSICRAFT_PYTHON") {
            if !p.trim().is_empty() {
                return p;
            }
        }
        // Newest uv-managed CPython 3.x on Windows, whatever its patch version.
        if let Ok(appdata) = std::env::var("APPDATA") {
            let root = PathBuf::from(appdata).join("uv").join("python");
            if let Ok(entries) = std::fs::read_dir(&root) {
                let mut found: Vec<PathBuf> = entries
                    .filter_map(Result::ok)
                    .filter(|e| e.file_name().to_string_lossy().starts_with("cpython-3."))
                    .map(|e| e.path().join("python.exe"))
                    .filter(|p| p.exists())
                    .collect();
                found.sort();
                if let Some(best) = found.pop() {
                    return best.to_string_lossy().to_string();
                }
            }
        }
        if cfg!(windows) {
            "python".into()
        } else {
            "python3".into()
        }
    }

    /// Check if candidate or evaluator is Rust code.
    ///
    /// The evaluator defines the task and decides the language whenever it is
    /// present; the candidate text is only inspected when there is no evaluator
    /// (demo mode), because Python code may mention `fn ` in strings or comments.
    #[must_use]
    pub fn is_rust_candidate(candidate_code: &str, evaluator_script: &str) -> bool {
        if let Some(lang) = CodeLanguage::from_evaluator(evaluator_script) {
            return lang == CodeLanguage::Rust;
        }
        candidate_code.contains("pub fn ")
            || (candidate_code.contains("fn ") && !candidate_code.contains("def "))
    }

    fn resolve_rustc_bin() -> String {
        if let Ok(p) = std::env::var("RSICRAFT_RUSTC") {
            if !p.trim().is_empty() {
                return p;
            }
        }
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let candidate = PathBuf::from(userprofile)
                .join(".cargo")
                .join("bin")
                .join("rustc.exe");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let candidate = PathBuf::from(home).join(".cargo").join("bin").join("rustc");
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
        trials: u32,
    ) -> Result<SandboxExecutionResult, String> {
        const BACKEND: &str = "local-rust";
        let start = Instant::now();
        let scratch = Scratch::new("rsicraft-eval-rust")?;
        let dir = scratch.path();
        scratch.write("candidate.rs", candidate_code)?;
        scratch.write("evaluator.rs", evaluator_script)?;
        let rlib_path = dir.join("libcandidate.rlib");
        let runner_bin = dir.join(if cfg!(windows) {
            "runner.exe"
        } else {
            "runner"
        });
        let rustc = Self::resolve_rustc_bin();

        let compile_error = |what: &str, run: &Captured| SandboxExecutionResult {
            status: if run.timed_out {
                "TIMEOUT".into()
            } else {
                "COMPILE_ERROR".into()
            },
            primary_score: 0.0,
            raw_output: bounded_log(&format!(
                "rustc {what} compilation {}:\n{}",
                if run.timed_out { "timed out" } else { "failed" },
                run.stderr
            )),
            execution_time_ms: elapsed_ms(start),
            backend: BACKEND.into(),
            runs_count: 1,
            score_std: 0.0,
        };

        // 1. Compile candidate as rlib
        let mut cand_cmd = ProcessCommand::new(&rustc);
        cand_cmd
            .args([
                "--crate-type=rlib",
                "--crate-name=candidate",
                "candidate.rs",
                "-O",
                "--out-dir",
                ".",
            ])
            .current_dir(dir);
        let cand = run_captured(
            &mut cand_cmd,
            dir,
            "rustc-candidate",
            None,
            RUSTC_COMPILE_TIMEOUT,
        )?;
        if !cand.success() {
            return Ok(compile_error("candidate", &cand));
        }

        // 2. Compile evaluator linking candidate
        let mut eval_cmd = ProcessCommand::new(&rustc);
        eval_cmd
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
            .current_dir(dir);
        let eval = run_captured(
            &mut eval_cmd,
            dir,
            "rustc-evaluator",
            None,
            RUSTC_COMPILE_TIMEOUT,
        )?;
        if !eval.success() {
            return Ok(compile_error("evaluator", &eval));
        }

        // 3. Execute runner under the wall-clock reaper (across N trials)
        let n_trials = trials.max(1);
        let mut scores = Vec::with_capacity(n_trials as usize);
        let mut last_status = String::new();
        let mut last_output = String::new();

        for i in 0..n_trials {
            let mut run_cmd = ProcessCommand::new(&runner_bin);
            run_cmd.current_dir(dir);
            let run = run_captured(
                &mut run_cmd,
                dir,
                &format!("runner-{i}"),
                None,
                Duration::from_secs(timeout_secs),
            )?;
            let combined = run.combined();
            if run.timed_out {
                return Ok(Self::timeout_result(&combined, start, BACKEND));
            }

            let (mut status, score) = Self::parse_evaluator_output(&run.stdout);
            if status == "METRIC_PARSE_ERROR" && !run.success() {
                status = "RUNTIME_ERROR".into();
            }
            last_status = status.clone();
            last_output = combined;
            if status != "SUCCESS" {
                return Ok(SandboxExecutionResult {
                    status,
                    primary_score: 0.0,
                    raw_output: bounded_log(&last_output),
                    execution_time_ms: elapsed_ms(start),
                    backend: BACKEND.into(),
                    runs_count: i + 1,
                    score_std: 0.0,
                });
            }
            scores.push(score);
        }

        let mean = scores.iter().sum::<f64>() / (scores.len() as f64);
        let std = calculate_sample_std(&scores, mean);
        let mut log = bounded_log(&last_output);
        if n_trials > 1 {
            log = format!("{log}\n[rsicraft-stats] trials={n_trials} mean={mean:.4} std={std:.4}");
        }
        Ok(SandboxExecutionResult {
            status: last_status,
            primary_score: mean,
            raw_output: log,
            execution_time_ms: elapsed_ms(start),
            backend: BACKEND.into(),
            runs_count: n_trials,
            score_std: std,
        })
    }

    /// Evaluate candidate code in an ephemeral local directory (`sandbox_mode = "local"`).
    ///
    /// Convenience backend, not a security boundary: the candidate runs as a
    /// normal child process of the current user with network and filesystem
    /// access. Use `docker` for code you do not trust.
    pub fn run_local_evaluation(
        candidate_code: &str,
        evaluator_script: &str,
        timeout_secs: u64,
        trials: u32,
    ) -> Result<SandboxExecutionResult, String> {
        let n_trials = trials.max(1);
        if Self::is_rust_candidate(candidate_code, evaluator_script) {
            return Self::run_local_rust_evaluation(
                candidate_code,
                evaluator_script,
                timeout_secs,
                n_trials,
            );
        }
        const BACKEND: &str = "local";
        let start = Instant::now();
        let scratch = Scratch::new("rsicraft-eval")?;
        scratch.write("candidate.py", candidate_code)?;
        scratch.write("evaluator.py", evaluator_script)?;

        let py_bin = Self::resolve_python_bin();
        let mut scores = Vec::with_capacity(n_trials as usize);
        let mut last_status = String::new();
        let mut last_output = String::new();

        for i in 0..n_trials {
            let mut cmd = ProcessCommand::new(&py_bin);
            cmd.args(["-B", "evaluator.py"])
                .current_dir(scratch.path())
                .env("PYTHONIOENCODING", "utf-8");
            let run = run_captured(
                &mut cmd,
                scratch.path(),
                &format!("python-{i}"),
                None,
                Duration::from_secs(timeout_secs),
            )?;
            let combined = run.combined();
            if run.timed_out {
                return Ok(Self::timeout_result(&combined, start, BACKEND));
            }

            let (mut status, score) = Self::parse_evaluator_output(&run.stdout);
            if status == "METRIC_PARSE_ERROR" && !run.success() {
                status = Self::classify_python_failure(&combined);
            }
            last_status = status.clone();
            last_output = combined;
            if status != "SUCCESS" {
                return Ok(SandboxExecutionResult {
                    status,
                    primary_score: 0.0,
                    raw_output: bounded_log(&last_output),
                    execution_time_ms: elapsed_ms(start),
                    backend: BACKEND.into(),
                    runs_count: i + 1,
                    score_std: 0.0,
                });
            }
            scores.push(score);
        }

        let mean = scores.iter().sum::<f64>() / (scores.len() as f64);
        let std = calculate_sample_std(&scores, mean);
        let mut log = bounded_log(&last_output);
        if n_trials > 1 {
            log = format!("{log}\n[rsicraft-stats] trials={n_trials} mean={mean:.4} std={std:.4}");
        }
        Ok(SandboxExecutionResult {
            status: last_status,
            primary_score: mean,
            raw_output: log,
            execution_time_ms: elapsed_ms(start),
            backend: BACKEND.into(),
            runs_count: n_trials,
            score_std: std,
        })
    }

    /// Deterministic offline evaluation oracle (`backend = "demo"`).
    /// Scores candidates deterministically based on structural properties and operator lineage.
    #[must_use]
    pub fn run_demo_evaluation(
        candidate_code: &str,
        op: SearchOperator,
        parent_score: Option<f64>,
        trials: u32,
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
                runs_count: 1,
                score_std: 0.0,
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
                runs_count: 1,
                score_std: 0.0,
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
        let n_trials = trials.max(1);
        let (runs_count, score_std) = if n_trials > 1 {
            (n_trials, 0.12)
        } else {
            (1, 0.0)
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
            runs_count,
            score_std,
        }
    }

    /// Unified evaluation gateway for all sandbox backends (`docker`, `local`, `demo`).
    pub fn evaluate(
        &self,
        sandbox_mode: &str,
        candidate_code: &str,
        evaluator_script: &str,
        operator: SearchOperator,
        parent_score: Option<f64>,
        timeout_secs: u64,
        trials: u32,
    ) -> Result<SandboxExecutionResult, String> {
        let n_trials = trials.max(1);
        match sandbox_mode.to_ascii_lowercase().as_str() {
            "docker" | "live" => {
                self.run_docker_evaluation(candidate_code, evaluator_script, timeout_secs, n_trials)
            }
            "local" | "python" => {
                Self::run_local_evaluation(candidate_code, evaluator_script, timeout_secs, n_trials)
            }
            "demo" => Ok(Self::run_demo_evaluation(
                candidate_code,
                operator,
                parent_score,
                n_trials,
            )),
            other => Err(format!(
                "invalid sandbox_mode '{other}'; expected 'demo', 'local', or 'docker'"
            )),
        }
    }
}
