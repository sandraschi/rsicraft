//! `rsicraft-core`: L1 core engine and Universal `Command::dispatch` Bus.
//!
//! Coordinates the 4-operator evolutionary loop (`DRAFT`, `IMPROVE`, `DEBUG`,
//! `CROSSOVER`), Frontis-MA1 4090 LLM inference, isolated sandbox evaluation,
//! and the Stigmergic SQLite WAL store.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]

pub mod bandit;
pub mod help;
pub mod llm;
pub mod operators;
pub mod sandbox;
pub mod skills;
pub mod store;
pub mod trainer;

#[cfg(test)]
mod tests;

use llm::{goliath_4090_profile, LlamaClient, DEFAULT_LLAMA_URL, DEFAULT_MODEL_ALIAS};
use operators::{CodeLanguage, PromptBuilder, SearchOperator};
use sandbox::{tail_str, SandboxExecutionResult, SandboxRunner, DEFAULT_DOCKER_IMAGE};
use serde_json::{json, Value};
pub use store::utility::UtilityHyperparams;
pub use store::{NewCardInput, Registry};

pub const APP: &str = "rsicraft";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Upper bound for the parent's sandbox log quoted in a DEBUG prompt. The tail
/// is kept because that is where the traceback ends.
const MAX_PROMPT_LOG_BYTES: usize = 6000;

/// Largest LCS table (`differing lines of A` x `differing lines of B`) the diff
/// will allocate. Beyond it the differing region is reported as one replaced block.
const MAX_DIFF_CELLS: usize = 4_000_000;

pub struct Ctx {
    pub reg: Registry,
    pub llm: LlamaClient,
    pub sandbox: SandboxRunner,
    pub hyperparams: UtilityHyperparams,
}

impl Ctx {
    #[must_use]
    pub fn new(
        reg: Registry,
        llama_url: Option<&str>,
        model_alias: Option<&str>,
        docker_image: Option<&str>,
    ) -> Self {
        // Explicit arguments win, then RSICRAFT_LLM_URL / RSICRAFT_MODEL, then defaults.
        let env = |key: &str| std::env::var(key).ok().filter(|v| !v.trim().is_empty());
        let url = llama_url
            .map(str::to_string)
            .or_else(|| env("RSICRAFT_LLM_URL"))
            .unwrap_or_else(|| DEFAULT_LLAMA_URL.to_string());
        let model = model_alias
            .map(str::to_string)
            .or_else(|| env("RSICRAFT_MODEL"))
            .unwrap_or_else(|| DEFAULT_MODEL_ALIAS.to_string());
        let img = docker_image.unwrap_or(DEFAULT_DOCKER_IMAGE);
        let hyperparams = reg.load_hyperparams();
        Self {
            reg,
            llm: LlamaClient::new(url, model),
            sandbox: SandboxRunner::new(img),
            hyperparams,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Status,
    HardwareProfile,
    TaskUpsert {
        task_id: String,
        name: String,
        description: String,
        evaluator_script: String,
    },
    TaskList,
    TaskShow {
        task_id: String,
    },
    Step {
        task_id: String,
        operator: SearchOperator,
        provider: String,
        sandbox_mode: String,
        timeout_secs: u64,
    },
    Campaign {
        task_id: String,
        steps: u32,
        provider: String,
        sandbox_mode: String,
        timeout_secs: u64,
    },
    GetFrontier {
        task_id: String,
        limit: usize,
    },
    CardList {
        task_id: String,
        operator: Option<String>,
        status: Option<String>,
        limit: usize,
    },
    CardShow {
        card_id: String,
    },
    Lineage {
        task_id: String,
    },
    Evaluate {
        task_id: String,
        code: String,
        operator: SearchOperator,
        parent_a_id: Option<String>,
        parent_b_id: Option<String>,
        sandbox_mode: String,
        timeout_secs: u64,
        record: bool,
    },
    DemoSeed,
    Help {
        tab: Option<String>,
        search: Option<String>,
    },
    SkillRead {
        name: Option<String>,
    },
    Diff {
        card_a_id: String,
        card_b_id: String,
    },
    LlmProbe {
        url: Option<String>,
    },
    HyperparamsGet,
    HyperparamsSet {
        w_score: Option<f64>,
        w_delta: Option<f64>,
        w_novelty: Option<f64>,
    },
    TaskDelete {
        task_id: String,
    },
    CardsPrune {
        task_id: String,
        keep_frontier: usize,
    },
    Export {
        task_id: String,
    },
    Import {
        bundle: Value,
    },
    DatasetExport {
        task_id: String,
        format: String,
        min_delta: f64,
    },
    BanditStatus {
        task_id: String,
    },
    TrainStep {
        task_id: String,
        epochs: u32,
        learning_rate: f64,
        dry_run: bool,
    },
    PromoteGate {
        task_id: String,
        trials: u32,
        steps: u32,
        sandbox_mode: String,
        timeout_secs: u64,
        baseline_alias: Option<String>,
        candidate_alias: Option<String>,
    },
}

fn req_str(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing or empty required string argument '{key}'"))
}

fn opt_str(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn opt_f64(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(Value::as_f64)
}

fn opt_u64(args: &Value, key: &str, default: u64) -> u64 {
    args.get(key).and_then(Value::as_u64).unwrap_or(default)
}

fn opt_usize(args: &Value, key: &str, default: usize) -> usize {
    args.get(key)
        .and_then(Value::as_u64)
        .and_then(|v| usize::try_from(v).ok())
        .unwrap_or(default)
}

/// Unified line-by-line diff comparing two code snippets.
/// Returns `(diff_text, lines_added, lines_removed)`.
#[must_use]
pub fn generate_unified_diff(
    a_name: &str,
    a_code: &str,
    b_name: &str,
    b_code: &str,
) -> (String, usize, usize) {
    let a_lines: Vec<&str> = a_code.lines().collect();
    let b_lines: Vec<&str> = b_code.lines().collect();

    if a_code == b_code {
        return (
            format!("--- {a_name}\n+++ {b_name}\n(identical content)\n"),
            0,
            0,
        );
    }

    // Only the lines between the common prefix and the common suffix need the
    // quadratic LCS table; a typical parent/child pair differs in a few lines.
    let (a_all, b_all) = (a_lines, b_lines);
    let mut prefix = 0;
    while prefix < a_all.len() && prefix < b_all.len() && a_all[prefix] == b_all[prefix] {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < a_all.len() - prefix
        && suffix < b_all.len() - prefix
        && a_all[a_all.len() - 1 - suffix] == b_all[b_all.len() - 1 - suffix]
    {
        suffix += 1;
    }
    let a_lines = &a_all[prefix..a_all.len() - suffix];
    let b_lines = &b_all[prefix..b_all.len() - suffix];
    let oversized = a_lines.len().saturating_mul(b_lines.len()) > MAX_DIFF_CELLS;
    let (n, m) = if oversized {
        (0, 0)
    } else {
        (a_lines.len(), b_lines.len())
    };

    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in 0..n {
        for j in 0..m {
            if a_lines[i] == b_lines[j] {
                dp[i + 1][j + 1] = dp[i][j].saturating_add(1);
            } else {
                dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut i = n;
    let mut j = m;
    let mut edits: Vec<(char, &str)> = Vec::new();
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && a_lines[i - 1] == b_lines[j - 1] {
            edits.push((' ', a_lines[i - 1]));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            edits.push(('+', b_lines[j - 1]));
            j -= 1;
        } else if i > 0 {
            edits.push(('-', a_lines[i - 1]));
            i -= 1;
        }
    }
    edits.reverse();
    if oversized {
        edits.extend(a_lines.iter().map(|l| ('-', *l)));
        edits.extend(b_lines.iter().map(|l| ('+', *l)));
    }
    let mut full: Vec<(char, &str)> = Vec::with_capacity(prefix + edits.len() + suffix);
    full.extend(a_all[..prefix].iter().map(|l| (' ', *l)));
    full.extend(edits);
    full.extend(a_all[a_all.len() - suffix..].iter().map(|l| (' ', *l)));
    let edits = full;

    let mut out = String::new();
    out.push_str(&format!("--- {a_name}\n"));
    out.push_str(&format!("+++ {b_name}\n"));
    let mut added = 0;
    let mut removed = 0;
    for (op, line) in edits {
        match op {
            '+' => {
                added += 1;
                out.push('+');
                out.push_str(line);
                out.push('\n');
            }
            '-' => {
                removed += 1;
                out.push('-');
                out.push_str(line);
                out.push('\n');
            }
            ' ' => {
                out.push(' ');
                out.push_str(line);
                out.push('\n');
            }
            _ => {}
        }
    }
    (out, added, removed)
}

impl Command {
    pub fn from_tool(name: &str, args: &Value) -> Result<Self, String> {
        let norm = name.replace('.', "_");
        match norm.as_str() {
            "rsi_status" => Ok(Self::Status),
            "rsi_hardware_profile" => Ok(Self::HardwareProfile),
            "rsi_task_upsert" => Ok(Self::TaskUpsert {
                task_id: req_str(args, "task_id")?,
                name: req_str(args, "name")?,
                description: req_str(args, "description")?,
                evaluator_script: req_str(args, "evaluator_script")?,
            }),
            "rsi_task_list" => Ok(Self::TaskList),
            "rsi_task_show" => Ok(Self::TaskShow {
                task_id: req_str(args, "task_id")?,
            }),
            "rsi_step" => {
                let op_raw = opt_str(args, "operator").unwrap_or_else(|| "DRAFT".into());
                Ok(Self::Step {
                    task_id: opt_str(args, "task_id").unwrap_or_else(|| "opt-synth".into()),
                    operator: SearchOperator::parse(&op_raw)?,
                    provider: opt_str(args, "provider").unwrap_or_else(|| "demo".into()),
                    sandbox_mode: opt_str(args, "sandbox_mode").unwrap_or_else(|| "demo".into()),
                    timeout_secs: opt_u64(args, "timeout_secs", 30).clamp(1, 600),
                })
            }
            "rsi_campaign" => {
                let steps = u32::try_from(opt_u64(args, "steps", 4).clamp(1, 64)).unwrap_or(4);
                Ok(Self::Campaign {
                    task_id: opt_str(args, "task_id").unwrap_or_else(|| "opt-synth".into()),
                    steps,
                    provider: opt_str(args, "provider").unwrap_or_else(|| "demo".into()),
                    sandbox_mode: opt_str(args, "sandbox_mode").unwrap_or_else(|| "demo".into()),
                    timeout_secs: opt_u64(args, "timeout_secs", 30).clamp(1, 600),
                })
            }
            "rsi_get_frontier" => Ok(Self::GetFrontier {
                task_id: opt_str(args, "task_id").unwrap_or_else(|| "opt-synth".into()),
                limit: opt_usize(args, "limit", 10).clamp(1, 100),
            }),
            "rsi_card_list" => Ok(Self::CardList {
                task_id: opt_str(args, "task_id").unwrap_or_else(|| "opt-synth".into()),
                operator: opt_str(args, "operator"),
                status: opt_str(args, "status"),
                limit: opt_usize(args, "limit", 50).clamp(1, 500),
            }),
            "rsi_card_show" => Ok(Self::CardShow {
                card_id: req_str(args, "card_id")?,
            }),
            "rsi_lineage" => Ok(Self::Lineage {
                task_id: opt_str(args, "task_id").unwrap_or_else(|| "opt-synth".into()),
            }),
            "rsi_evaluate" => {
                let op_raw = opt_str(args, "operator").unwrap_or_else(|| "DRAFT".into());
                let record = args.get("record").and_then(Value::as_bool).unwrap_or(true);
                Ok(Self::Evaluate {
                    task_id: req_str(args, "task_id")?,
                    code: req_str(args, "code")?,
                    operator: SearchOperator::parse(&op_raw)?,
                    parent_a_id: opt_str(args, "parent_a_id"),
                    parent_b_id: opt_str(args, "parent_b_id"),
                    sandbox_mode: opt_str(args, "sandbox_mode").unwrap_or_else(|| "demo".into()),
                    timeout_secs: opt_u64(args, "timeout_secs", 30).clamp(1, 600),
                    record,
                })
            }
            "rsi_demo_seed" => Ok(Self::DemoSeed),
            "rsi_help" => Ok(Self::Help {
                tab: opt_str(args, "tab"),
                search: opt_str(args, "search"),
            }),
            "rsicraft_skill_read" => Ok(Self::SkillRead {
                name: opt_str(args, "name"),
            }),
            "rsi_diff" => Ok(Self::Diff {
                card_a_id: req_str(args, "card_a_id")?,
                card_b_id: req_str(args, "card_b_id")?,
            }),
            "rsi_llm_probe" => Ok(Self::LlmProbe {
                url: opt_str(args, "url"),
            }),
            "rsi_hyperparams_get" => Ok(Self::HyperparamsGet),
            "rsi_hyperparams_set" => Ok(Self::HyperparamsSet {
                w_score: opt_f64(args, "w_score"),
                w_delta: opt_f64(args, "w_delta"),
                w_novelty: opt_f64(args, "w_novelty"),
            }),
            "rsi_task_delete" => Ok(Self::TaskDelete {
                task_id: req_str(args, "task_id")?,
            }),
            "rsi_cards_prune" => Ok(Self::CardsPrune {
                task_id: opt_str(args, "task_id").unwrap_or_else(|| "opt-synth".into()),
                keep_frontier: opt_usize(args, "keep_frontier", 10).clamp(1, 100),
            }),
            "rsi_export" => Ok(Self::Export {
                task_id: opt_str(args, "task_id").unwrap_or_else(|| "opt-synth".into()),
            }),
            "rsi_import" => {
                let bundle = if let Some(obj) = args.get("bundle") {
                    obj.clone()
                } else if let Some(s) = args.get("bundle_json").and_then(Value::as_str) {
                    serde_json::from_str(s)
                        .map_err(|e| format!("invalid JSON in bundle_json: {e}"))?
                } else {
                    return Err(
                        "missing 'bundle' (object) or 'bundle_json' (string) argument".into(),
                    );
                };
                Ok(Self::Import { bundle })
            }
            "rsi_dataset_export" => Ok(Self::DatasetExport {
                task_id: opt_str(args, "task_id").unwrap_or_else(|| "opt-synth".into()),
                format: opt_str(args, "format").unwrap_or_else(|| "all".into()),
                min_delta: opt_f64(args, "min_delta").unwrap_or(0.0),
            }),
            "rsi_bandit_status" => Ok(Self::BanditStatus {
                task_id: opt_str(args, "task_id").unwrap_or_else(|| "opt-synth".into()),
            }),
            "rsi_train_step" => {
                let epochs = u32::try_from(opt_u64(args, "epochs", 3).clamp(1, 100)).unwrap_or(3);
                let dry_run = args
                    .get("dry_run")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let lr = opt_f64(args, "learning_rate").unwrap_or(0.0002);
                Ok(Self::TrainStep {
                    task_id: opt_str(args, "task_id").unwrap_or_else(|| "opt-synth".into()),
                    epochs,
                    learning_rate: lr,
                    dry_run,
                })
            }
            "rsi_promote_gate" => {
                let trials = u32::try_from(opt_u64(args, "trials", 5).clamp(3, 20)).unwrap_or(5);
                Ok(Self::PromoteGate {
                    task_id: opt_str(args, "task_id").unwrap_or_else(|| "opt-synth".into()),
                    trials,
                    steps: u32::try_from(opt_u64(args, "steps", 6).clamp(1, 64)).unwrap_or(6),
                    sandbox_mode: opt_str(args, "sandbox_mode").unwrap_or_else(|| "local".into()),
                    timeout_secs: opt_u64(args, "timeout_secs", 30).clamp(1, 600),
                    baseline_alias: opt_str(args, "baseline_alias"),
                    candidate_alias: opt_str(args, "candidate_alias"),
                })
            }
            other => Err(format!("unknown tool '{other}'")),
        }
    }
}

fn tool_def(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
        }
    })
}

#[must_use]
pub fn tools() -> Vec<Value> {
    vec![
        tool_def(
            "rsi_status",
            "Return stigmergic SQLite store statistics, card counts, and Goliath RTX 4090 summary.",
            json!({}),
            &[],
        ),
        tool_def(
            "rsi_hardware_profile",
            "Return the Goliath RTX 4090 (24 GB VRAM) + Frontis-MA1 30B/35B MoE hardware profile and llama-server launch command.",
            json!({}),
            &[],
        ),
        tool_def(
            "rsi_task_upsert",
            "Create or update an evolutionary optimization task and its sandbox evaluator script.",
            json!({
                "task_id": { "type": "string", "description": "Unique task identifier (e.g. 'opt-synth')." },
                "name": { "type": "string", "description": "Human-readable task title." },
                "description": { "type": "string", "description": "Problem specification passed to evolutionary prompts." },
                "evaluator_script": { "type": "string", "description": "Python evaluator script printing RSI_METRIC_JSON={\"status\":...,\"score\":...}." }
            }),
            &["task_id", "name", "description", "evaluator_script"],
        ),
        tool_def(
            "rsi_task_list",
            "List all registered optimization tasks.",
            json!({}),
            &[],
        ),
        tool_def(
            "rsi_task_show",
            "Show a task, its score/delta bounds, and its current top frontier.",
            json!({
                "task_id": { "type": "string" }
            }),
            &["task_id"],
        ),
        tool_def(
            "rsi_step",
            "Execute a single evolutionary step (DRAFT, IMPROVE, DEBUG, or CROSSOVER) with composite-utility parent selection, LLM synthesis, sandbox evaluation, and BLAKE3+Zstd CAS persistence.",
            json!({
                "task_id": { "type": "string", "description": "Target task ID (default: 'opt-synth')." },
                "operator": { "type": "string", "enum": ["DRAFT", "IMPROVE", "DEBUG", "CROSSOVER"] },
                "provider": { "type": "string", "enum": ["demo", "llama"], "description": "'llama' queries Frontis-MA1 on llama-server; 'demo' uses deterministic oracle." },
                "sandbox_mode": { "type": "string", "enum": ["demo", "local", "docker"], "description": "'docker' runs an isolated container; 'local' runs Python or rustc directly on the host (no isolation, trusted code only); 'demo' runs a deterministic oracle." },
                "timeout_secs": { "type": "integer", "description": "Sandbox execution timeout in seconds (default 30)." }
            }),
            &[],
        ),
        tool_def(
            "rsi_campaign",
            "Run a multi-step autonomous evolutionary campaign cycling across DRAFT, IMPROVE, DEBUG, and CROSSOVER.",
            json!({
                "task_id": { "type": "string" },
                "steps": { "type": "integer", "description": "Number of evolutionary steps to execute (1..=64)." },
                "provider": { "type": "string", "enum": ["demo", "llama"] },
                "sandbox_mode": { "type": "string", "enum": ["demo", "local", "docker"] },
                "timeout_secs": { "type": "integer" }
            }),
            &[],
        ),
        tool_def(
            "rsi_get_frontier",
            "Get the top-K materialized frontier experience cards for a task.",
            json!({
                "task_id": { "type": "string" },
                "limit": { "type": "integer" }
            }),
            &[],
        ),
        tool_def(
            "rsi_card_list",
            "List experience cards for a task ordered by composite utility, optionally filtered by operator or execution status.",
            json!({
                "task_id": { "type": "string" },
                "operator": { "type": "string", "enum": ["DRAFT", "IMPROVE", "DEBUG", "CROSSOVER"] },
                "status": { "type": "string", "enum": ["SUCCESS", "COMPILE_ERROR", "RUNTIME_ERROR", "TIMEOUT", "METRIC_PARSE_ERROR"] },
                "limit": { "type": "integer" }
            }),
            &[],
        ),
        tool_def(
            "rsi_card_show",
            "Show full details of one experience card including decompressed source code, BLAKE3 hash, execution logs, and lineage.",
            json!({
                "card_id": { "type": "string" }
            }),
            &["card_id"],
        ),
        tool_def(
            "rsi_lineage",
            "Return the evolutionary lineage DAG (nodes and parent->child edges) for a task.",
            json!({
                "task_id": { "type": "string" }
            }),
            &[],
        ),
        tool_def(
            "rsi_evaluate",
            "Directly evaluate a candidate Python code snippet against a task's evaluator and optionally record it as an experience card.",
            json!({
                "task_id": { "type": "string" },
                "code": { "type": "string" },
                "operator": { "type": "string", "enum": ["DRAFT", "IMPROVE", "DEBUG", "CROSSOVER"] },
                "parent_a_id": { "type": "string" },
                "parent_b_id": { "type": "string" },
                "sandbox_mode": { "type": "string", "enum": ["demo", "local", "docker"] },
                "timeout_secs": { "type": "integer" },
                "record": { "type": "boolean" }
            }),
            &["task_id", "code"],
        ),
        tool_def(
            "rsi_demo_seed",
            "Seed an empty database with the spec's 'opt-synth' (Vector Max) benchmark task and a 4-card evolutionary progression.",
            json!({}),
            &[],
        ),
        tool_def(
            "rsi_help",
            "Read the rsicraft manual: start (what this is), rsi (what recursive self-improvement is and where rsicraft stands), based_on (Frontis-MA1/OpenRSI and prior work), quickstart, providers (Ollama, LM Studio, llama-server, hosted), tasks (evaluator contract), concepts (operators, selection, sandboxes), api (all tools and CLI verbs), troubleshoot (every [llm:<class>] error with its fix), faq. Optional substring search.",
            json!({
                "tab": {
                    "type": "string",
                    "enum": ["all", "start", "rsi", "based_on", "quickstart", "providers", "tasks", "concepts", "api", "troubleshoot", "faq"],
                    "description": "Help tab to display (default: 'all')."
                },
                "search": {
                    "type": "string",
                    "description": "Optional case-insensitive substring query across all help tabs and sections."
                }
            }),
            &[],
        ),
        tool_def(
            "rsi_diff",
            "Generate a unified text diff comparing decompressed source codes between two experience cards (e.g. parent vs child).",
            json!({
                "card_a_id": { "type": "string", "description": "Base/parent card ID." },
                "card_b_id": { "type": "string", "description": "Candidate/child card ID to compare against card A." }
            }),
            &["card_a_id", "card_b_id"],
        ),
        tool_def(
            "rsi_llm_probe",
            "Actively ping the configured LLM endpoint (/v1/models), returning reachability status, latency in ms, and active model alias.",
            json!({
                "url": { "type": "string", "description": "Optional custom llama-server URL to probe (defaults to configured base_url)." }
            }),
            &[],
        ),
        tool_def(
            "rsi_hyperparams_get",
            "Get current composite utility weights (w_score, w_delta, w_novelty) used for stochastic parent selection.",
            json!({}),
            &[],
        ),
        tool_def(
            "rsi_hyperparams_set",
            "Set composite utility weights (w_score, w_delta, w_novelty). The weights are persisted in the store and every task is re-ranked immediately.",
            json!({
                "w_score": { "type": "number", "description": "Weight for normalized primary task score." },
                "w_delta": { "type": "number", "description": "Weight for delta improvement over parent score." },
                "w_novelty": { "type": "number", "description": "Weight for exploration novelty (reciprocal of child count)." }
            }),
            &[],
        ),
        tool_def(
            "rsi_task_delete",
            "Delete a task and cascade delete all its associated experience cards and frontier records.",
            json!({
                "task_id": { "type": "string", "description": "ID of task to permanently delete." }
            }),
            &["task_id"],
        ),
        tool_def(
            "rsi_cards_prune",
            "Prune low-utility non-frontier experience cards for a task, keeping at least keep_frontier top cards.",
            json!({
                "task_id": { "type": "string", "description": "Task ID whose non-frontier cards should be pruned." },
                "keep_frontier": { "type": "integer", "description": "Minimum top cards to retain in frontier (default 10)." }
            }),
            &["task_id"],
        ),
        tool_def(
            "rsi_export",
            "Export a complete task lineage bundle (task definition, decompressed CAS cards, and Pareto frontier) to JSON.",
            json!({
                "task_id": { "type": "string", "description": "Task ID to export." }
            }),
            &["task_id"],
        ),
        tool_def(
            "rsi_import",
            "Import a complete task lineage bundle from JSON into the stigmergic SQLite store.",
            json!({
                "bundle": { "type": "object", "description": "Parsed task bundle JSON object." },
                "bundle_json": { "type": "string", "description": "Stringified task bundle JSON." }
            }),
            &[],
        ),
        tool_def(
            "rsi_dataset_export",
            "Rung 2 Self-Evolution: Export curated SFT trajectories (positive deltas, error repairs) and DPO contrastive preference pairs from the task DAG.",
            json!({
                "task_id": { "type": "string", "description": "Task ID to extract learning traces from (default: 'opt-synth')." },
                "format": { "type": "string", "enum": ["all", "sft", "dpo"], "description": "Dataset export format (default: 'all')." },
                "min_delta": { "type": "number", "description": "Minimum score improvement delta required to include an SFT trace (default: 0.0)." }
            }),
            &[],
        ),
        tool_def(
            "rsi_bandit_status",
            "Advisory operator statistics for a task: per-operator pulls, wins, Beta posterior parameters and a recommended next operator. Not Thompson sampling yet and not used by campaigns.",
            json!({
                "task_id": { "type": "string", "description": "Task ID to inspect bandit state for." }
            }),
            &[],
        ),
        tool_def(
            "rsi_train_step",
            "Post-train a LoRA adapter on the task's curated SFT traces with sandboxes/train_lora.py (needs RSICRAFT_TRAIN_BASE_MODEL and a CUDA Python environment). Produces a candidate adapter only; the active model changes only through rsi_promote_gate. Nothing is simulated: it trains or it fails.",
            json!({
                "task_id": { "type": "string", "description": "Task ID to post-train on (default: 'opt-synth')." },
                "epochs": { "type": "integer", "description": "Number of fine-tuning epochs (1..=100, default: 3)." },
                "learning_rate": { "type": "number", "description": "LoRA learning rate (default: 0.0002)." },
                "dry_run": { "type": "boolean", "description": "Report how many traces would be trained and change nothing." }
            }),
            &[],
        ),
        tool_def(
            "rsi_promote_gate",
            "A/B gate between two model aliases served by the LLM endpoint: runs `trials` fresh searches of `steps` steps per model on scratch copies of the task and promotes the candidate only if its best scores are clearly higher (Welch t >= 2). Needs a live endpoint and a real sandbox; can take a long time.",
            json!({
                "task_id": { "type": "string", "description": "Task ID to evaluate (default: 'opt-synth')." },
                "trials": { "type": "integer", "description": "Fresh searches per model (3..=20, default 5)." },
                "steps": { "type": "integer", "description": "Search steps per trial (1..=64, default 6)." },
                "sandbox_mode": { "type": "string", "enum": ["local", "docker"], "description": "Sandbox backend (default local)." },
                "timeout_secs": { "type": "integer", "description": "Sandbox timeout per evaluation (default 30)." },
                "baseline_alias": { "type": "string", "description": "Baseline model alias (defaults to the task's current model)." },
                "candidate_alias": { "type": "string", "description": "Required. Alias under which the endpoint serves the model to test." }
            }),
            &["candidate_alias"],
        ),
        tool_def(
            "rsicraft_skill_read",
            "Read a bundled RSICraft SKILL.md guide (e.g. 'rsicraft-workflow', 'rsicraft-ergonomics') or list all bundled skills.",
            json!({
                "name": {
                    "type": "string",
                    "description": "Skill name or skill:// URI (omit or pass 'list' to list available bundled skills)."
                }
            }),
            &[],
        ),
    ]
}

// ---------------------------------------------------------------------------
// Three-phase step pipeline: plan (store read) -> run (no store) -> commit
// (store write). Only the middle phase is slow (LLM inference and sandbox
// execution) and it borrows nothing from the store, so a GUI can run it on a
// worker thread while the single SQLite connection stays on its own thread.
// ---------------------------------------------------------------------------

/// Everything one evolutionary step needs once its parents are chosen.
/// Owns all of its data, so it can be moved to another thread.
#[derive(Debug, Clone)]
pub struct StepPlan {
    pub task_id: String,
    pub operator: SearchOperator,
    pub provider: String,
    pub sandbox_mode: String,
    pub timeout_secs: u64,
    pub temperature: f64,
    pub evaluator_script: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub parent_a_id: Option<String>,
    pub parent_b_id: Option<String>,
    pub parent_score: Option<f64>,
    /// Model alias this step sends to the LLM endpoint.
    pub model_alias: String,
    pub model_generation: u32,
    /// 1-based index of the card this step will create (demo provider only).
    pub demo_step_index: u32,
}

/// Result of the slow phase of a step: the synthesized candidate and its verdict.
#[derive(Debug, Clone)]
pub struct StepOutcome {
    pub candidate_code: String,
    pub eval: SandboxExecutionResult,
}

/// A direct evaluation (`rsi_evaluate`) with its store lookups already done.
#[derive(Debug, Clone)]
pub struct EvalPlan {
    pub task_id: String,
    pub code: String,
    pub operator: SearchOperator,
    pub parent_a_id: Option<String>,
    pub parent_b_id: Option<String>,
    pub sandbox_mode: String,
    pub timeout_secs: u64,
    pub record: bool,
    pub evaluator_script: String,
    pub parent_score: Option<f64>,
    pub model_generation: u32,
}

fn normalize_sandbox_mode(raw: &str) -> Result<&'static str, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "docker" | "live" => Ok("docker"),
        "local" | "python" => Ok("local"),
        "demo" => Ok("demo"),
        other => Err(format!(
            "invalid sandbox_mode '{other}'; expected 'demo', 'local', or 'docker'"
        )),
    }
}

fn normalize_provider(raw: &str) -> Result<&'static str, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "llama" | "live" | "frontis" => Ok("llama"),
        "demo" => Ok("demo"),
        other => Err(format!(
            "invalid provider '{other}'; expected 'demo' or 'llama'"
        )),
    }
}

/// Model alias a task's steps are sent to. Generation 0 uses the alias the
/// process was configured with (`--model`); a promoted generation uses the
/// alias recorded on the task.
#[must_use]
pub fn effective_model_alias(ctx: &Ctx, task: &store::TaskRecord) -> String {
    if task.active_model_gen > 0 && !task.active_model_alias.trim().is_empty() {
        task.active_model_alias.clone()
    } else {
        ctx.llm.model_alias.clone()
    }
}

/// Phase 1: validate the request, select parents by composite utility and
/// build the prompts. Reads the store, writes nothing.
pub fn plan_step(
    ctx: &Ctx,
    task_id: &str,
    operator: SearchOperator,
    provider: &str,
    sandbox_mode: &str,
    timeout_secs: u64,
) -> Result<StepPlan, String> {
    // Reject bad modes before any inference is spent on the step.
    let provider = normalize_provider(provider)?;
    let sandbox_mode = normalize_sandbox_mode(sandbox_mode)?;
    let task = ctx
        .reg
        .get_task(task_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("task '{task_id}' not found"))?;

    let (parent_a, parent_b) = match operator {
        SearchOperator::Draft => (None, None),
        SearchOperator::Improve => {
            let pa = ctx
                .reg
                .sample_parent_by_utility(task_id, false, None)
                .map_err(|e| e.to_string())?;
            (pa, None)
        }
        SearchOperator::Debug => {
            let pa = ctx
                .reg
                .sample_parent_by_utility(task_id, true, None)
                .map_err(|e| e.to_string())?;
            (pa, None)
        }
        SearchOperator::Crossover => {
            let pa = ctx
                .reg
                .sample_parent_by_utility(task_id, false, None)
                .map_err(|e| e.to_string())?;
            let exclude = pa.as_ref().map(|c| c.card_id.as_str());
            let pb = ctx
                .reg
                .sample_parent_by_utility(task_id, false, exclude)
                .map_err(|e| e.to_string())?;
            if pa.is_none() || pb.is_none() {
                return Err(format!(
                    "CROSSOVER needs two distinct cards in task '{task_id}'; run DRAFT or IMPROVE first"
                ));
            }
            (pa, pb)
        }
    };
    if operator != SearchOperator::Draft && parent_a.is_none() {
        return Err(format!(
            "{operator} needs an existing card in task '{task_id}'; run DRAFT first"
        ));
    }

    let lang = CodeLanguage::from_evaluator(&task.evaluator_script).unwrap_or(CodeLanguage::Python);
    let system_prompt = PromptBuilder::build_system_prompt(operator, lang);
    let user_prompt = PromptBuilder::build_user_prompt(
        operator,
        lang,
        &task.description,
        parent_a.as_ref().map(|p| p.code.as_str()),
        parent_b.as_ref().map(|p| p.code.as_str()),
        parent_a
            .as_ref()
            .map(|p| tail_str(&p.raw_output, MAX_PROMPT_LOG_BYTES)),
    );
    let bounds = ctx
        .reg
        .get_task_score_bounds(task_id)
        .map_err(|e| e.to_string())?;

    Ok(StepPlan {
        task_id: task_id.to_string(),
        operator,
        provider: provider.to_string(),
        sandbox_mode: sandbox_mode.to_string(),
        timeout_secs,
        temperature: operator.target_temperature(),
        model_alias: effective_model_alias(ctx, &task),
        model_generation: task.active_model_gen,
        evaluator_script: task.evaluator_script,
        system_prompt,
        user_prompt,
        parent_score: parent_a.as_ref().map(|p| p.primary_score),
        parent_a_id: parent_a.map(|p| p.card_id),
        parent_b_id: parent_b.map(|p| p.card_id),
        demo_step_index: bounds.total_cards.saturating_add(1),
    })
}

/// Phase 2: synthesize the candidate (LLM or demo oracle) and evaluate it in
/// the sandbox. This is the slow phase and it touches no store.
pub fn run_step(
    plan: &StepPlan,
    llm: &LlamaClient,
    sandbox: &SandboxRunner,
) -> Result<StepOutcome, String> {
    let raw_llm_text = if plan.provider == "llama" {
        let mut client = llm.clone();
        client.model_alias.clone_from(&plan.model_alias);
        client.complete(&plan.system_prompt, &plan.user_prompt, plan.temperature)?
    } else {
        LlamaClient::demo_synthesize(plan.operator, &plan.task_id, plan.demo_step_index)
    };
    let candidate_code = PromptBuilder::extract_code_block(&raw_llm_text);
    if plan.provider == "llama" {
        // Never evaluate prose or fragments as if they were a candidate: report
        // the step as producing no code, with the reason.
        let lang =
            CodeLanguage::from_evaluator(&plan.evaluator_script).unwrap_or(CodeLanguage::Python);
        if !PromptBuilder::looks_like_program(&candidate_code, lang) {
            let preview: String = candidate_code.chars().take(80).collect();
            return Err(llm::LlmError::new(
                llm::LlmFailure::NoCode,
                format!(
                    "the answer has no top-level {} function ({} characters, starting '{}')",
                    lang.name(),
                    candidate_code.len(),
                    preview.replace('\n', " ")
                ),
                "the model may be too weak for this task, or it ran out of budget mid-answer: try a stronger model or a larger RSICRAFT_MAX_TOKENS",
            )
            .into());
        }
    }
    let eval = sandbox.evaluate(
        &plan.sandbox_mode,
        &candidate_code,
        &plan.evaluator_script,
        plan.operator,
        plan.parent_score,
        plan.timeout_secs,
        1,
    )?;
    Ok(StepOutcome {
        candidate_code,
        eval,
    })
}

/// Phase 3: persist the experience card and refresh utilities and frontier.
pub fn commit_step(ctx: &Ctx, plan: &StepPlan, outcome: &StepOutcome) -> Result<Value, String> {
    let card_id = uuid::Uuid::new_v4().to_string();
    let card = ctx
        .reg
        .record_experience_card(
            &NewCardInput {
                card_id: &card_id,
                task_id: &plan.task_id,
                parent_a_id: plan.parent_a_id.as_deref(),
                parent_b_id: plan.parent_b_id.as_deref(),
                operator_used: plan.operator.as_str(),
                code: &outcome.candidate_code,
                execution_status: &outcome.eval.status,
                raw_output: &outcome.eval.raw_output,
                primary_score: outcome.eval.primary_score,
                score_std: outcome.eval.score_std,
                runs_count: outcome.eval.runs_count,
                model_generation: plan.model_generation,
                execution_time_ms: outcome.eval.execution_time_ms,
            },
            &ctx.hyperparams,
        )
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "event": "NEW_CARD",
        "card_id": card.card_id,
        "task_id": card.task_id,
        "operator": card.operator_used,
        "temperature": plan.temperature,
        "provider": plan.provider,
        "model_alias": if plan.provider == "llama" { Some(plan.model_alias.as_str()) } else { None },
        "model_generation": plan.model_generation,
        "sandbox_backend": outcome.eval.backend,
        "status": card.execution_status,
        "primary_score": card.primary_score,
        "delta_score": card.delta_score,
        "novelty_score": card.novelty_score,
        "composite_utility": card.composite_utility,
        "code_hash": card.code_hash,
        "parent_a_id": card.parent_a_id,
        "parent_b_id": card.parent_b_id,
        "execution_time_ms": card.execution_time_ms,
    }))
}

fn execute_step(
    ctx: &mut Ctx,
    task_id: &str,
    operator: SearchOperator,
    provider: &str,
    sandbox_mode: &str,
    timeout_secs: u64,
) -> Result<Value, String> {
    let plan = plan_step(ctx, task_id, operator, provider, sandbox_mode, timeout_secs)?;
    let outcome = run_step(&plan, &ctx.llm, &ctx.sandbox)?;
    commit_step(ctx, &plan, &outcome)
}

/// Operator a campaign runs at `step_index` (0-based). The fixed DRAFT, IMPROVE,
/// DEBUG, CROSSOVER cycle adapts to what the task can support: an empty task
/// starts with DRAFT, and DEBUG or CROSSOVER slots fall back to IMPROVE when
/// there is no failed card to repair or fewer than two cards to recombine.
pub fn campaign_operator(
    ctx: &Ctx,
    task_id: &str,
    step_index: u32,
) -> Result<SearchOperator, String> {
    const SCHEDULE: [SearchOperator; 4] = [
        SearchOperator::Draft,
        SearchOperator::Improve,
        SearchOperator::Debug,
        SearchOperator::Crossover,
    ];
    let bounds = ctx
        .reg
        .get_task_score_bounds(task_id)
        .map_err(|e| e.to_string())?;
    let failed_cards = bounds.total_cards.saturating_sub(bounds.success_cards);
    Ok(match SCHEDULE[(step_index as usize) % SCHEDULE.len()] {
        _ if bounds.total_cards == 0 => SearchOperator::Draft,
        SearchOperator::Debug if failed_cards == 0 => SearchOperator::Improve,
        SearchOperator::Crossover if bounds.total_cards < 2 => SearchOperator::Improve,
        scheduled => scheduled,
    })
}

/// Phase 1 of a direct evaluation: resolve the task evaluator and parent score.
#[allow(clippy::too_many_arguments)]
pub fn plan_evaluate(
    ctx: &Ctx,
    task_id: &str,
    code: &str,
    operator: SearchOperator,
    parent_a_id: Option<&str>,
    parent_b_id: Option<&str>,
    sandbox_mode: &str,
    timeout_secs: u64,
    record: bool,
) -> Result<EvalPlan, String> {
    let sandbox_mode = normalize_sandbox_mode(sandbox_mode)?;
    let task = ctx
        .reg
        .get_task(task_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("task '{task_id}' not found"))?;
    let parent_score = match parent_a_id {
        Some(pid) => ctx
            .reg
            .get_card(pid)
            .map_err(|e| e.to_string())?
            .map(|c| c.primary_score),
        None => None,
    };
    Ok(EvalPlan {
        task_id: task_id.to_string(),
        code: code.to_string(),
        operator,
        parent_a_id: parent_a_id.map(str::to_string),
        parent_b_id: parent_b_id.map(str::to_string),
        sandbox_mode: sandbox_mode.to_string(),
        timeout_secs,
        record,
        model_generation: task.active_model_gen,
        evaluator_script: task.evaluator_script,
        parent_score,
    })
}

/// Phase 2 of a direct evaluation: the sandbox run. Touches no store.
pub fn run_evaluate(
    plan: &EvalPlan,
    sandbox: &SandboxRunner,
) -> Result<SandboxExecutionResult, String> {
    sandbox.evaluate(
        &plan.sandbox_mode,
        &plan.code,
        &plan.evaluator_script,
        plan.operator,
        plan.parent_score,
        plan.timeout_secs,
        1,
    )
}

/// Phase 3 of a direct evaluation: return the verdict, recording it as an
/// experience card first when the plan asks for that.
pub fn commit_evaluate(
    ctx: &Ctx,
    plan: &EvalPlan,
    result: &SandboxExecutionResult,
) -> Result<Value, String> {
    if !plan.record {
        return serde_json::to_value(result).map_err(|e| e.to_string());
    }
    let card_id = uuid::Uuid::new_v4().to_string();
    let card = ctx
        .reg
        .record_experience_card(
            &NewCardInput {
                card_id: &card_id,
                task_id: &plan.task_id,
                parent_a_id: plan.parent_a_id.as_deref(),
                parent_b_id: plan.parent_b_id.as_deref(),
                operator_used: plan.operator.as_str(),
                code: &plan.code,
                execution_status: &result.status,
                raw_output: &result.raw_output,
                primary_score: result.primary_score,
                score_std: result.score_std,
                runs_count: result.runs_count,
                model_generation: plan.model_generation,
                execution_time_ms: result.execution_time_ms,
            },
            &ctx.hyperparams,
        )
        .map_err(|e| e.to_string())?;
    serde_json::to_value(card).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Promotion gate: a real A/B comparison of two model aliases.
// ---------------------------------------------------------------------------

/// Best frontier score one model reaches in a fresh `steps`-step search on a
/// scratch copy of the task. The scratch task is always deleted afterwards.
fn gate_trial(
    ctx: &mut Ctx,
    task: &store::TaskRecord,
    alias: &str,
    steps: u32,
    sandbox_mode: &str,
    timeout_secs: u64,
) -> Result<f64, String> {
    let scratch = format!("gate-{}", uuid::Uuid::new_v4());
    ctx.reg
        .upsert_task(
            &scratch,
            &task.name,
            &task.description,
            &task.evaluator_script,
        )
        .map_err(|e| e.to_string())?;
    let run = (|| -> Result<f64, String> {
        // Generation 1 pins the scratch task to exactly this alias.
        ctx.reg
            .set_task_active_model(&scratch, 1, alias)
            .map_err(|e| e.to_string())?;
        for idx in 0..steps {
            let op = campaign_operator(ctx, &scratch, idx)?;
            execute_step(ctx, &scratch, op, "llama", sandbox_mode, timeout_secs)?;
        }
        let frontier = ctx.reg.get_frontier(&scratch).map_err(|e| e.to_string())?;
        Ok(frontier.first().map_or(0.0, |f| f.card.primary_score))
    })();
    let _ = ctx.reg.delete_task(&scratch);
    run
}

fn mean_and_var(xs: &[f64]) -> (f64, f64) {
    if xs.is_empty() {
        return (0.0, 0.0);
    }
    #[allow(clippy::cast_precision_loss)]
    let n = xs.len() as f64;
    let mean = xs.iter().sum::<f64>() / n;
    let var = if xs.len() < 2 {
        0.0
    } else {
        xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)
    };
    (mean, var)
}

/// Welch t statistic for `candidate` against `baseline` (positive favours the candidate).
#[must_use]
pub fn welch_t(baseline: &[f64], candidate: &[f64]) -> f64 {
    let (mb, vb) = mean_and_var(baseline);
    let (mc, vc) = mean_and_var(candidate);
    #[allow(clippy::cast_precision_loss)]
    let se = (vb / baseline.len().max(1) as f64 + vc / candidate.len().max(1) as f64).sqrt();
    if se <= f64::EPSILON {
        return match (mc - mb).partial_cmp(&0.0) {
            Some(std::cmp::Ordering::Greater) => f64::INFINITY,
            Some(std::cmp::Ordering::Less) => f64::NEG_INFINITY,
            _ => 0.0,
        };
    }
    (mc - mb) / se
}

/// A campaign stops after this many steps in a row that produced no card.
pub const CAMPAIGN_MAX_CONSECUTIVE_FAILURES: u32 = 2;

/// One-sided threshold on the Welch statistic. 2.0 is deliberately stricter
/// than the large-sample 5% value (1.645) because gates run with few trials.
pub const GATE_T_THRESHOLD: f64 = 2.0;

/// Compare a candidate model against the baseline by running `trials` fresh
/// searches of `steps` steps each per model on scratch copies of the task, and
/// promote the candidate only when its best-score distribution is clearly
/// higher. Needs a live LLM endpoint serving both aliases.
#[allow(clippy::too_many_arguments)]
fn execute_promote_gate(
    ctx: &mut Ctx,
    task_id: &str,
    trials: u32,
    steps: u32,
    sandbox_mode: &str,
    timeout_secs: u64,
    baseline_alias: Option<&str>,
    candidate_alias: Option<&str>,
) -> Result<Value, String> {
    let sandbox_mode = normalize_sandbox_mode(sandbox_mode)?;
    if sandbox_mode == "demo" {
        return Err(
            "the promotion gate needs a real sandbox ('local' or 'docker'): the demo oracle scores candidates from the operator name and cannot tell two models apart".into(),
        );
    }
    let task = ctx
        .reg
        .get_task(task_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("task '{task_id}' not found"))?;
    let candidate = candidate_alias.ok_or_else(|| {
        "candidate_alias is required: the alias under which the LLM endpoint serves the model to test".to_string()
    })?;
    let baseline = baseline_alias
        .map(str::to_string)
        .unwrap_or_else(|| effective_model_alias(ctx, &task));
    if baseline == candidate {
        return Err(format!(
            "baseline and candidate are the same alias '{baseline}'"
        ));
    }
    let trials = trials.clamp(3, 20);

    let mut baseline_scores = Vec::new();
    let mut candidate_scores = Vec::new();
    // Interleave the two arms so drift on the host affects both equally.
    for _ in 0..trials {
        baseline_scores.push(gate_trial(
            ctx,
            &task,
            &baseline,
            steps,
            sandbox_mode,
            timeout_secs,
        )?);
        candidate_scores.push(gate_trial(
            ctx,
            &task,
            candidate,
            steps,
            sandbox_mode,
            timeout_secs,
        )?);
    }

    let (mb, _) = mean_and_var(&baseline_scores);
    let (mc, _) = mean_and_var(&candidate_scores);
    let t = welch_t(&baseline_scores, &candidate_scores);
    let promoted = t >= GATE_T_THRESHOLD;
    let active_alias = if promoted {
        ctx.reg
            .set_task_active_model(task_id, task.active_model_gen.saturating_add(1), candidate)
            .map_err(|e| e.to_string())?;
        candidate.to_string()
    } else {
        effective_model_alias(ctx, &task)
    };
    Ok(json!({
        "task_id": task_id,
        "baseline_alias": baseline,
        "candidate_alias": candidate,
        "trials": trials,
        "steps_per_trial": steps,
        "sandbox_mode": sandbox_mode,
        "baseline_scores": baseline_scores,
        "candidate_scores": candidate_scores,
        "baseline_mean_score": mb,
        "candidate_mean_score": mc,
        "score_delta": mc - mb,
        "welch_t": if t.is_finite() { Some(t) } else { None },
        "t_threshold": GATE_T_THRESHOLD,
        "promoted": promoted,
        "active_model_alias": active_alias,
        "verdict": if promoted {
            format!("PROMOTED: '{candidate}' beat '{baseline}' (mean {mc:.4} vs {mb:.4}, t={t:.2})")
        } else {
            format!("KEPT BASELINE: '{candidate}' did not clearly beat '{baseline}' (mean {mc:.4} vs {mb:.4}, t={t:.2}, need t>={GATE_T_THRESHOLD})")
        },
    }))
}
pub fn dispatch(ctx: &mut Ctx, cmd: Command) -> Result<Value, String> {
    match cmd {
        Command::Status => {
            let st = ctx.reg.status().map_err(|e| e.to_string())?;
            let hw = goliath_4090_profile(&ctx.llm.base_url);
            Ok(json!({
                "app": APP,
                "version": VERSION,
                "journal_mode": st.journal_mode,
                "tasks_count": st.tasks_count,
                "cards_count": st.cards_count,
                "success_cards_count": st.success_cards_count,
                "frontier_entries_count": st.frontier_entries_count,
                "unique_code_hashes": st.unique_code_hashes,
                "gpu": hw.gpu,
                "vram_total_gb": hw.vram_total_gb,
                "model": hw.model_architecture,
                "llama_url": ctx.llm.base_url,
                "docker_image": ctx.sandbox.image_tag,
            }))
        }
        Command::HardwareProfile => {
            let hw = goliath_4090_profile(&ctx.llm.base_url);
            serde_json::to_value(hw).map_err(|e| e.to_string())
        }
        Command::TaskUpsert {
            task_id,
            name,
            description,
            evaluator_script,
        } => {
            let rec = ctx
                .reg
                .upsert_task(&task_id, &name, &description, &evaluator_script)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(rec).map_err(|e| e.to_string())
        }
        Command::TaskList => {
            let tasks = ctx.reg.list_tasks().map_err(|e| e.to_string())?;
            Ok(json!({
                "count": tasks.len(),
                "tasks": tasks,
            }))
        }
        Command::TaskShow { task_id } => {
            let task = ctx
                .reg
                .get_task(&task_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("task '{task_id}' not found"))?;
            let bounds = ctx
                .reg
                .get_task_score_bounds(&task_id)
                .map_err(|e| e.to_string())?;
            let frontier = ctx.reg.get_frontier(&task_id).map_err(|e| e.to_string())?;
            Ok(json!({
                "task": task,
                "bounds": bounds,
                "frontier": frontier,
            }))
        }
        Command::Step {
            task_id,
            operator,
            provider,
            sandbox_mode,
            timeout_secs,
        } => execute_step(
            ctx,
            &task_id,
            operator,
            &provider,
            &sandbox_mode,
            timeout_secs,
        ),
        Command::Campaign {
            task_id,
            steps,
            provider,
            sandbox_mode,
            timeout_secs,
        } => {
            // Check the provider once before spending any steps: a wrong address,
            // an unknown model or a missing key fails here with a fix, not as a
            // string of skipped steps.
            let mut provider_report = Value::Null;
            if normalize_provider(&provider)? == "llama" {
                let task = ctx
                    .reg
                    .get_task(&task_id)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| format!("task '{task_id}' not found"))?;
                let mut client = ctx.llm.clone();
                client.model_alias = effective_model_alias(ctx, &task);
                provider_report = client.preflight().map_err(String::from)?;
            }
            let mut results = Vec::new();
            let mut failures = Vec::new();
            let mut failures_by_kind = serde_json::Map::new();
            let mut aborted: Option<String> = None;
            let mut consecutive_failures = 0u32;
            for idx in 0..steps {
                let op = campaign_operator(ctx, &task_id, idx)?;
                match execute_step(ctx, &task_id, op, &provider, &sandbox_mode, timeout_secs) {
                    Ok(step_val) => {
                        consecutive_failures = 0;
                        results.push(step_val);
                    }
                    Err(e) => {
                        // A step that produced no card is recorded with its class.
                        // A setup failure (fatal class) stops the campaign at once;
                        // otherwise two failed steps in a row stop it.
                        let kind = llm::failure_kind_of(&e);
                        let kind_name = kind.map_or("other", llm::LlmFailure::as_str);
                        let count = failures_by_kind
                            .get(kind_name)
                            .and_then(Value::as_u64)
                            .unwrap_or(0);
                        failures_by_kind.insert(kind_name.to_string(), json!(count + 1));
                        consecutive_failures += 1;
                        failures.push(json!({
                            "step": idx,
                            "operator": op.as_str(),
                            "kind": kind_name,
                            "error": e,
                        }));
                        let fatal = kind.is_some_and(llm::LlmFailure::is_fatal);
                        if fatal || consecutive_failures >= CAMPAIGN_MAX_CONSECUTIVE_FAILURES {
                            if results.is_empty() {
                                return Err(e);
                            }
                            aborted = Some(e);
                            break;
                        }
                    }
                }
            }
            let frontier = ctx.reg.get_frontier(&task_id).map_err(|e| e.to_string())?;
            let best_score = frontier.first().map_or(0.0, |f| f.card.primary_score);
            Ok(json!({
                "task_id": task_id,
                "steps_requested": steps,
                "steps_executed": results.len(),
                "failures_by_kind": failures_by_kind,
                "provider": provider_report,                "steps_skipped": failures.len(),
                "skipped": failures,
                "aborted": aborted.is_some(),
                "error": aborted,
                "best_score": best_score,
                "steps": results,
            }))
        }
        Command::GetFrontier { task_id, limit } => {
            ctx.reg
                .refresh_task_frontier(&task_id, limit)
                .map_err(|e| e.to_string())?;
            let entries = ctx.reg.get_frontier(&task_id).map_err(|e| e.to_string())?;
            let rows: Vec<Value> = entries
                .iter()
                .map(|e| {
                    json!({
                        "rank": e.rank,
                        "card_id": e.card.card_id,
                        "operator": e.card.operator_used,
                        "status": e.card.execution_status,
                        "primary_score": e.card.primary_score,
                        "delta_score": e.card.delta_score,
                        "novelty_score": e.card.novelty_score,
                        "composite_utility": e.card.composite_utility,
                        "children_count": e.card.children_count,
                        "execution_time_ms": e.card.execution_time_ms,
                        "code_hash": e.card.code_hash,
                        "code": e.card.code,
                    })
                })
                .collect();
            Ok(json!({
                "task_id": task_id,
                "count": rows.len(),
                "frontier": rows,
            }))
        }
        Command::CardList {
            task_id,
            operator,
            status,
            limit,
        } => {
            let cards = ctx
                .reg
                .list_cards(&task_id, operator.as_deref(), status.as_deref(), limit)
                .map_err(|e| e.to_string())?;
            let rows: Vec<Value> = cards
                .iter()
                .map(|c| {
                    json!({
                        "card_id": c.card_id,
                        "operator": c.operator_used,
                        "status": c.execution_status,
                        "primary_score": c.primary_score,
                        "delta_score": c.delta_score,
                        "novelty_score": c.novelty_score,
                        "composite_utility": c.composite_utility,
                        "children_count": c.children_count,
                        "execution_time_ms": c.execution_time_ms,
                        "parent_a_id": c.parent_a_id,
                        "parent_b_id": c.parent_b_id,
                        "code_hash": c.code_hash,
                    })
                })
                .collect();
            Ok(json!({
                "task_id": task_id,
                "count": rows.len(),
                "cards": rows,
            }))
        }
        Command::CardShow { card_id } => {
            let card = ctx
                .reg
                .get_card(&card_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("card '{card_id}' not found"))?;
            serde_json::to_value(card).map_err(|e| e.to_string())
        }
        Command::Lineage { task_id } => {
            let cards = ctx
                .reg
                .list_cards(&task_id, None, None, 500)
                .map_err(|e| e.to_string())?;
            let nodes: Vec<Value> = cards
                .iter()
                .map(|c| {
                    json!({
                        "card_id": c.card_id,
                        "operator": c.operator_used,
                        "status": c.execution_status,
                        "primary_score": c.primary_score,
                        "composite_utility": c.composite_utility,
                        "children_count": c.children_count,
                    })
                })
                .collect();
            let mut edges = Vec::new();
            for c in &cards {
                if let Some(pa) = &c.parent_a_id {
                    edges.push(json!({
                        "from": pa,
                        "to": c.card_id,
                        "role": "parent_a",
                        "operator": c.operator_used,
                    }));
                }
                if let Some(pb) = &c.parent_b_id {
                    edges.push(json!({
                        "from": pb,
                        "to": c.card_id,
                        "role": "parent_b",
                        "operator": c.operator_used,
                    }));
                }
            }
            Ok(json!({
                "task_id": task_id,
                "nodes_count": nodes.len(),
                "edges_count": edges.len(),
                "nodes": nodes,
                "edges": edges,
            }))
        }
        Command::Evaluate {
            task_id,
            code,
            operator,
            parent_a_id,
            parent_b_id,
            sandbox_mode,
            timeout_secs,
            record,
        } => {
            let plan = plan_evaluate(
                ctx,
                &task_id,
                &code,
                operator,
                parent_a_id.as_deref(),
                parent_b_id.as_deref(),
                &sandbox_mode,
                timeout_secs,
                record,
            )?;
            let result = run_evaluate(&plan, &ctx.sandbox)?;
            commit_evaluate(ctx, &plan, &result)
        }
        Command::DemoSeed => {
            let st = ctx.reg.status().map_err(|e| e.to_string())?;
            if st.tasks_count > 0 || st.cards_count > 0 {
                return Err("demo_seed requires an empty database".into());
            }
            ctx.reg
                .upsert_task(
                    "opt-synth",
                    "Vector Max (DEMO)",
                    "Write solve(arr) returning the maximum numeric element in arr, or 0.0 when empty.",
                    "import json, candidate\nscore = float(candidate.solve([1, 5, 3])) * 19.0\nprint(f'RSI_METRIC_JSON={json.dumps({\"status\": \"SUCCESS\", \"score\": score})}')",
                )
                .map_err(|e| e.to_string())?;

            let c1 = execute_step(ctx, "opt-synth", SearchOperator::Draft, "demo", "demo", 30)?;
            let c2 = execute_step(ctx, "opt-synth", SearchOperator::Debug, "demo", "demo", 30)?;
            let c3 = execute_step(
                ctx,
                "opt-synth",
                SearchOperator::Improve,
                "demo",
                "demo",
                30,
            )?;
            let c4 = execute_step(
                ctx,
                "opt-synth",
                SearchOperator::Crossover,
                "demo",
                "demo",
                30,
            )?;

            Ok(json!({
                "seeded": true,
                "task_id": "opt-synth",
                "cards_created": 4,
                "trajectory": [c1, c2, c3, c4],
            }))
        }
        Command::Help { tab, search } => help::query_help(tab.as_deref(), search.as_deref()),
        Command::SkillRead { name } => skills::read_skill_json(name.as_deref()),
        Command::Diff {
            card_a_id,
            card_b_id,
        } => {
            let card_a = ctx
                .reg
                .get_card(&card_a_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("card_a '{card_a_id}' not found"))?;
            let card_b = ctx
                .reg
                .get_card(&card_b_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("card_b '{card_b_id}' not found"))?;
            let (diff_text, added, removed) = generate_unified_diff(
                &format!("card_{card_a_id}.py"),
                &card_a.code,
                &format!("card_{card_b_id}.py"),
                &card_b.code,
            );
            Ok(json!({
                "card_a_id": card_a_id,
                "card_b_id": card_b_id,
                "card_a_operator": card_a.operator_used,
                "card_b_operator": card_b.operator_used,
                "card_a_score": card_a.primary_score,
                "card_b_score": card_b.primary_score,
                "diff_text": diff_text,
                "added_lines": added,
                "removed_lines": removed,
                "identical": added == 0 && removed == 0,
            }))
        }
        Command::LlmProbe { url } => {
            let client = match url {
                Some(u) => LlamaClient::new(u, &ctx.llm.model_alias),
                None => ctx.llm.clone(),
            };
            Ok(client.diagnose())
        }
        Command::HyperparamsGet => serde_json::to_value(ctx.hyperparams).map_err(|e| e.to_string()),
        Command::HyperparamsSet {
            w_score,
            w_delta,
            w_novelty,
        } => {
            if let Some(s) = w_score {
                if s < 0.0 {
                    return Err("w_score must be non-negative".into());
                }
                ctx.hyperparams.lambda_s = s;
            }
            if let Some(d) = w_delta {
                if d < 0.0 {
                    return Err("w_delta must be non-negative".into());
                }
                ctx.hyperparams.lambda_delta = d;
            }
            if let Some(n) = w_novelty {
                if n < 0.0 {
                    return Err("w_novelty must be non-negative".into());
                }
                ctx.hyperparams.lambda_nu = n;
            }
            let sum =
                ctx.hyperparams.lambda_s + ctx.hyperparams.lambda_delta + ctx.hyperparams.lambda_nu;
            if sum <= 0.0 {
                return Err("sum of hyperparameter weights must be strictly positive".into());
            }
            ctx.reg
                .save_hyperparams(&ctx.hyperparams)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(ctx.hyperparams).map_err(|e| e.to_string())
        }
        Command::TaskDelete { task_id } => {
            let deleted = ctx.reg.delete_task(&task_id).map_err(|e| e.to_string())?;
            Ok(json!({
                "task_id": task_id,
                "deleted": deleted,
            }))
        }
        Command::CardsPrune {
            task_id,
            keep_frontier,
        } => {
            let pruned = ctx
                .reg
                .prune_cards(&task_id, keep_frontier)
                .map_err(|e| e.to_string())?;
            let remaining = ctx
                .reg
                .list_cards(&task_id, None, None, 10_000)
                .map_err(|e| e.to_string())?
                .len();
            Ok(json!({
                "task_id": task_id,
                "cards_pruned": pruned,
                "cards_remaining": remaining,
            }))
        }
        Command::Export { task_id } => ctx
            .reg
            .export_task_bundle(&task_id)
            .map_err(|e| e.to_string()),
        Command::Import { bundle } => {
            let count = ctx
                .reg
                .import_task_bundle(&bundle, &ctx.hyperparams)
                .map_err(|e| e.to_string())?;
            Ok(json!({
                "imported_cards": count,
            }))
        }
        Command::DatasetExport {
            task_id,
            format,
            min_delta,
        } => {
            let summary = ctx
                .reg
                .get_dataset_summary(&task_id)
                .map_err(|e| e.to_string())?;
            let sft = ctx
                .reg
                .export_sft_traces(&task_id, min_delta)
                .map_err(|e| e.to_string())?;
            let dpo = ctx
                .reg
                .export_dpo_traces(&task_id)
                .map_err(|e| e.to_string())?;
            Ok(json!({
                "task_id": task_id,
                "format": format,
                "summary": summary,
                "sft_traces": sft,
                "dpo_traces": dpo,
            }))
        }
        Command::BanditStatus { task_id } => {
            let state = ctx
                .reg
                .get_bandit_state(&task_id)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(state).map_err(|e| e.to_string())
        }
        Command::TrainStep {
            task_id,
            epochs,
            learning_rate,
            dry_run,
        } => {
            let res =
                trainer::execute_train_step(&ctx.reg, &task_id, epochs, learning_rate, dry_run)?;
            serde_json::to_value(res).map_err(|e| e.to_string())
        }
        Command::PromoteGate {
            task_id,
            trials,
            steps,
            sandbox_mode,
            timeout_secs,
            baseline_alias,
            candidate_alias,
        } => execute_promote_gate(
            ctx,
            &task_id,
            trials,
            steps,
            &sandbox_mode,
            timeout_secs,
            baseline_alias.as_deref(),
            candidate_alias.as_deref(),
        ),
    }
}
