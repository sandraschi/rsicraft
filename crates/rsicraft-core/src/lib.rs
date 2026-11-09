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

pub mod help;
pub mod llm;
pub mod operators;
pub mod sandbox;
pub mod store;

#[cfg(test)]
mod tests;

use llm::{goliath_4090_profile, LlamaClient, DEFAULT_LLAMA_URL, DEFAULT_MODEL_ALIAS};
use operators::{PromptBuilder, SearchOperator};
use sandbox::{SandboxRunner, DEFAULT_DOCKER_IMAGE};
use serde_json::{json, Value};
pub use store::utility::UtilityHyperparams;
pub use store::{NewCardInput, Registry};

pub const APP: &str = "rsicraft";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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
        let url = llama_url.unwrap_or(DEFAULT_LLAMA_URL);
        let model = model_alias.unwrap_or(DEFAULT_MODEL_ALIAS);
        let img = docker_image.unwrap_or(DEFAULT_DOCKER_IMAGE);
        Self {
            reg,
            llm: LlamaClient::new(url, model),
            sandbox: SandboxRunner::new(img),
            hyperparams: UtilityHyperparams::default(),
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

    let n = a_lines.len();
    let m = b_lines.len();

    if a_code == b_code {
        return (
            format!("--- {a_name}\n+++ {b_name}\n(identical content)\n"),
            0,
            0,
        );
    }

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
                "sandbox_mode": { "type": "string", "enum": ["demo", "docker"], "description": "'docker' runs isolated container; 'demo' runs deterministic oracle." },
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
                "sandbox_mode": { "type": "string", "enum": ["demo", "docker"] },
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
                "sandbox_mode": { "type": "string", "enum": ["demo", "docker"] },
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
            "Query the 9-tab RSICraft Operator Manual, RSI Geopolitics ('Skynet' vs Closed Labs), Tsinghua/Frontis.AI OpenRSI Dossier, and Competitive Matrix (tabs: overview, rsi_landscape, tsinghua_openrsi, wrappee, operators, compare, api, troubleshoot, faq, all) with optional substring search.",
            json!({
                "tab": {
                    "type": "string",
                    "enum": ["all", "overview", "rsi_landscape", "tsinghua_openrsi", "wrappee", "operators", "compare", "api", "troubleshoot", "faq"],
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
            "Dynamically set composite utility weights (w_score, w_delta, w_novelty) at runtime.",
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
    ]
}

fn execute_step(
    ctx: &mut Ctx,
    task_id: &str,
    operator: SearchOperator,
    provider: &str,
    sandbox_mode: &str,
    timeout_secs: u64,
) -> Result<Value, String> {
    let task = ctx
        .reg
        .get_task(task_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("task '{task_id}' not found"))?;

    // 1. Select parents using composite utility
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
            (pa, pb)
        }
    };

    // 2. Build system & user prompts with target temperature
    let sys_prompt = PromptBuilder::build_system_prompt(operator);
    let user_prompt = PromptBuilder::build_user_prompt(
        operator,
        &task.description,
        parent_a.as_ref().map(|p| p.code.as_str()),
        parent_b.as_ref().map(|p| p.code.as_str()),
        parent_a.as_ref().map(|p| p.raw_output.as_str()),
    );
    let temperature = operator.target_temperature();

    let bounds = ctx
        .reg
        .get_task_score_bounds(task_id)
        .map_err(|e| e.to_string())?;

    // 3. Synthesize candidate via LLM or Demo oracle
    let raw_llm_text = match provider.to_ascii_lowercase().as_str() {
        "llama" | "live" | "frontis" => ctx.llm.complete(sys_prompt, &user_prompt, temperature)?,
        "demo" => {
            LlamaClient::demo_synthesize(operator, task_id, bounds.total_cards.saturating_add(1))
        }
        other => {
            return Err(format!(
                "invalid provider '{other}'; expected 'demo' or 'llama'"
            ))
        }
    };
    let candidate_code = PromptBuilder::extract_code_block(&raw_llm_text);

    // 4. Evaluate candidate inside sandbox
    let parent_score = parent_a.as_ref().map(|p| p.primary_score);
    let eval_result = match sandbox_mode.to_ascii_lowercase().as_str() {
        "docker" | "live" => ctx.sandbox.run_docker_evaluation(
            &candidate_code,
            &task.evaluator_script,
            timeout_secs,
        )?,
        "local" | "python" => SandboxRunner::run_local_evaluation(
            &candidate_code,
            &task.evaluator_script,
            timeout_secs,
        )?,
        "demo" => SandboxRunner::run_demo_evaluation(&candidate_code, operator, parent_score),
        other => {
            return Err(format!(
                "invalid sandbox_mode '{other}'; expected 'demo', 'local', or 'docker'"
            ))
        }
    };

    // 5. Persist experience card & refresh frontier
    let card_id = uuid::Uuid::new_v4().to_string();
    let card = ctx
        .reg
        .record_experience_card(
            &NewCardInput {
                card_id: &card_id,
                task_id,
                parent_a_id: parent_a.as_ref().map(|p| p.card_id.as_str()),
                parent_b_id: parent_b.as_ref().map(|p| p.card_id.as_str()),
                operator_used: operator.as_str(),
                code: &candidate_code,
                execution_status: &eval_result.status,
                raw_output: &eval_result.raw_output,
                primary_score: eval_result.primary_score,
                execution_time_ms: eval_result.execution_time_ms,
            },
            &ctx.hyperparams,
        )
        .map_err(|e| e.to_string())?;

    Ok(json!({
        "event": "NEW_CARD",
        "card_id": card.card_id,
        "task_id": card.task_id,
        "operator": card.operator_used,
        "temperature": temperature,
        "provider": provider,
        "sandbox_backend": eval_result.backend,
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
            let mut results = Vec::new();
            let schedule = [
                SearchOperator::Draft,
                SearchOperator::Improve,
                SearchOperator::Debug,
                SearchOperator::Crossover,
            ];
            for idx in 0..steps {
                let bounds = ctx
                    .reg
                    .get_task_score_bounds(&task_id)
                    .map_err(|e| e.to_string())?;
                let op = if bounds.total_cards == 0 {
                    SearchOperator::Draft
                } else {
                    schedule[(idx as usize) % schedule.len()]
                };
                let step_val =
                    execute_step(ctx, &task_id, op, &provider, &sandbox_mode, timeout_secs)?;
                results.push(step_val);
            }
            let frontier = ctx.reg.get_frontier(&task_id).map_err(|e| e.to_string())?;
            let best_score = frontier.first().map_or(0.0, |f| f.card.primary_score);
            Ok(json!({
                "task_id": task_id,
                "steps_executed": results.len(),
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
            let task = ctx
                .reg
                .get_task(&task_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("task '{task_id}' not found"))?;
            let parent_score = match parent_a_id.as_deref() {
                Some(pid) => ctx
                    .reg
                    .get_card(pid)
                    .map_err(|e| e.to_string())?
                    .map(|c| c.primary_score),
                None => None,
            };
            let eval_result = match sandbox_mode.to_ascii_lowercase().as_str() {
                "docker" | "live" => ctx.sandbox.run_docker_evaluation(
                    &code,
                    &task.evaluator_script,
                    timeout_secs,
                )?,
                "local" | "python" => SandboxRunner::run_local_evaluation(
                    &code,
                    &task.evaluator_script,
                    timeout_secs,
                )?,
                "demo" => SandboxRunner::run_demo_evaluation(&code, operator, parent_score),
                other => {
                    return Err(format!(
                        "invalid sandbox_mode '{other}'; expected 'demo', 'local', or 'docker'"
                    ))
                }
            };
            if !record {
                return serde_json::to_value(eval_result).map_err(|e| e.to_string());
            }
            let card_id = uuid::Uuid::new_v4().to_string();
            let card = ctx
                .reg
                .record_experience_card(
                    &NewCardInput {
                        card_id: &card_id,
                        task_id: &task_id,
                        parent_a_id: parent_a_id.as_deref(),
                        parent_b_id: parent_b_id.as_deref(),
                        operator_used: operator.as_str(),
                        code: &code,
                        execution_status: &eval_result.status,
                        raw_output: &eval_result.raw_output,
                        primary_score: eval_result.primary_score,
                        execution_time_ms: eval_result.execution_time_ms,
                    },
                    &ctx.hyperparams,
                )
                .map_err(|e| e.to_string())?;
            serde_json::to_value(card).map_err(|e| e.to_string())
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
            client.probe()
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
                .import_task_bundle(&bundle)
                .map_err(|e| e.to_string())?;
            Ok(json!({
                "imported_cards": count,
            }))
        }
    }
}
