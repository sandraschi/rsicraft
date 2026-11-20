//! `rsicraft`: Sovereign Recursive Self-Improvement Engine CLI and MCP server.
//!
//! Every CLI verb builds the exact same tool-argument JSON an MCP client sends
//! and routes through `Command::from_tool` + `dispatch`, guaranteeing 100%
//! CLI/MCP parity. In `mcp` mode stdout carries only JSON-RPC; logs go to stderr.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented
)]

mod mcp;

use clap::{Parser, Subcommand};
use mcp::{serve_stdio, ToolHost};
use rsicraft_core::{dispatch, tools, Command, Ctx, Registry, APP, VERSION};
use serde_json::{json, Map, Value};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "rsicraft",
    version,
    about = "Sovereign Recursive Self-Improvement Engine (Frontis-MA1 / OpenRSI on RTX 4090). CLI + MCP."
)]
struct Cli {
    /// SQLite database path. Default: $RSICRAFT_DB or %LOCALAPPDATA%\rsicraft\rsicraft.db
    #[arg(long, global = true)]
    db: Option<PathBuf>,
    /// OpenAI-compatible server root: Ollama http://127.0.0.1:11434, LM Studio http://127.0.0.1:1234, llama-server http://127.0.0.1:8080. Default: $RSICRAFT_LLM_URL or http://127.0.0.1:8081.
    #[arg(long, global = true)]
    llama_url: Option<String>,
    /// Model name as the server knows it (`ollama list`, LM Studio model id). Default: $RSICRAFT_MODEL or frontis.
    #[arg(long, global = true)]
    model: Option<String>,
    /// Docker evaluator image tag (default: rsicraft-evaluator:latest).
    #[arg(long, global = true)]
    docker_image: Option<String>,
    /// Emit machine-readable JSON only.
    #[arg(long, global = true)]
    json: bool,
    /// Open a throwaway or `--db` store and check status, hardware, help, and the tool count.
    #[arg(long, global = true)]
    smoke: bool,
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Store statistics, card counts, and Goliath RTX 4090 summary.
    Status,
    /// Show the Goliath RTX 4090 (24 GB VRAM) + Frontis-MA1 30B/35B MoE hardware profile.
    Hardware,
    /// Manage optimization tasks.
    Task {
        #[command(subcommand)]
        op: TaskOp,
    },
    /// Execute one evolutionary search step (DRAFT, IMPROVE, DEBUG, CROSSOVER).
    Step {
        #[arg(long, default_value = "opt-synth")]
        task_id: String,
        #[arg(long, default_value = "DRAFT")]
        operator: String,
        #[arg(long, default_value = "demo")]
        provider: String,
        #[arg(long, default_value = "demo")]
        sandbox_mode: String,
        #[arg(long, default_value_t = 30)]
        timeout_secs: u64,
    },
    /// Run a multi-step evolutionary campaign cycling across operators.
    Campaign {
        #[arg(long, default_value = "opt-synth")]
        task_id: String,
        #[arg(long, default_value_t = 4)]
        steps: u32,
        #[arg(long, default_value = "demo")]
        provider: String,
        #[arg(long, default_value = "demo")]
        sandbox_mode: String,
        #[arg(long, default_value_t = 30)]
        timeout_secs: u64,
    },
    /// Show the top-K materialized frontier for a task.
    Frontier {
        #[arg(long, default_value = "opt-synth")]
        task_id: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// List experience cards for a task ordered by composite utility.
    Cards {
        #[arg(long, default_value = "opt-synth")]
        task_id: String,
        #[arg(long)]
        operator: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Show full details and decompressed code of one experience card.
    Card { card_id: String },
    /// Show the evolutionary lineage DAG (nodes and parent->child edges) for a task.
    Lineage {
        #[arg(long, default_value = "opt-synth")]
        task_id: String,
    },
    /// Directly evaluate a candidate Python snippet against a task's evaluator.
    Evaluate {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        code: String,
        #[arg(long, default_value = "DRAFT")]
        operator: String,
        #[arg(long, default_value = "demo")]
        sandbox_mode: String,
        #[arg(long, default_value_t = 30)]
        timeout_secs: u64,
        #[arg(long)]
        no_record: bool,
    },
    /// Seed an empty database with the 'opt-synth' (Vector Max) demo benchmark and 4 evolutionary cards.
    DemoSeed,
    /// Compare source codes of two experience cards as a unified diff.
    Diff {
        #[arg(long)]
        card_a: String,
        #[arg(long)]
        card_b: String,
    },
    /// Check the LLM server and model: reachability, server type, served models, context settings. Scans the usual local addresses when nothing answers.
    Probe {
        #[arg(long)]
        url: Option<String>,
    },
    /// Inspect or update composite utility weights (w_score, w_delta, w_novelty).
    Hyperparams {
        #[arg(long)]
        w_score: Option<f64>,
        #[arg(long)]
        w_delta: Option<f64>,
        #[arg(long)]
        w_novelty: Option<f64>,
    },
    /// Prune low-utility non-frontier cards for a task.
    Prune {
        #[arg(long, default_value = "opt-synth")]
        task_id: String,
        #[arg(long, default_value_t = 10)]
        keep_frontier: usize,
    },
    /// Export complete task lineage (task, CAS cards, Pareto frontier) to JSON.
    Export {
        #[arg(long, default_value = "opt-synth")]
        task_id: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Import complete task lineage from JSON bundle file into SQLite.
    Import {
        #[arg(long)]
        file: PathBuf,
    },
    /// Export curated SFT trajectories and DPO preference pairs (Frontis Ladder Rung 2: Self-Evolution).
    DatasetExport {
        #[arg(long, default_value = "opt-synth")]
        task_id: String,
        #[arg(long, default_value = "all")]
        format: String,
        #[arg(long, default_value_t = 0.0)]
        min_delta: f64,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Advisory per-operator statistics and a recommended next operator (not used by campaigns).
    Bandit {
        #[arg(long, default_value = "opt-synth")]
        task_id: String,
    },
    /// Post-train a LoRA adapter on the curated traces of a task (candidate only; needs RSICRAFT_TRAIN_BASE_MODEL).
    Train {
        #[arg(long, default_value = "opt-synth")]
        task_id: String,
        #[arg(long, default_value_t = 3)]
        epochs: u32,
        #[arg(long, default_value_t = 0.0002)]
        learning_rate: f64,
        #[arg(long)]
        dry_run: bool,
    },
    /// A/B gate between baseline and candidate model aliases over fresh searches; promotes only on a clear win.
    Promote {
        #[arg(long, default_value = "opt-synth")]
        task_id: String,
        #[arg(long, default_value_t = 5)]
        trials: u32,
        #[arg(long, default_value_t = 6)]
        steps: u32,
        #[arg(long, default_value = "local")]
        sandbox_mode: String,
        #[arg(long, default_value_t = 30)]
        timeout_secs: u64,
        #[arg(long)]
        baseline: Option<String>,
        #[arg(long)]
        candidate: Option<String>,
    },
    /// Read the 10-tab manual (start, rsi, based_on, quickstart, providers, tasks, concepts, api, troubleshoot, faq).
    HelpTabs {
        #[arg(long, default_value = "all")]
        tab: String,
        #[arg(long)]
        search: Option<String>,
    },
    /// Print the MCP tool catalog as JSON.
    Tools,
    /// Run the MCP 2024-11-05 stdio server.
    Mcp,
}

#[derive(Subcommand)]
enum TaskOp {
    /// List all tasks.
    List,
    /// Show one task with score bounds and frontier.
    Show { task_id: String },
    /// Create or update a task.
    Upsert {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: String,
        /// Evaluator source as text (awkward in shells; prefer --evaluator-file).
        #[arg(
            long,
            conflicts_with = "evaluator_file",
            required_unless_present = "evaluator_file"
        )]
        evaluator_script: Option<String>,
        /// Read the evaluator from a file, e.g. tasks/pack-v1/primes.py.
        #[arg(long)]
        evaluator_file: Option<PathBuf>,
    },
    /// Delete a task and cascade delete its experience cards and frontier.
    Delete { task_id: String },
}

fn default_db() -> PathBuf {
    if let Ok(p) = std::env::var("RSICRAFT_DB") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("rsicraft").join("rsicraft.db")
}

fn obj(pairs: Vec<(&str, Option<Value>)>) -> Value {
    let mut m = Map::new();
    for (k, v) in pairs {
        if let Some(v) = v {
            m.insert(k.to_string(), v);
        }
    }
    Value::Object(m)
}

fn to_tool(cmd: Cmd) -> Result<(&'static str, Value), String> {
    Ok(match cmd {
        Cmd::Status => ("rsi_status", json!({})),
        Cmd::Hardware => ("rsi_hardware_profile", json!({})),
        Cmd::Task { op: TaskOp::List } => ("rsi_task_list", json!({})),
        Cmd::Task {
            op: TaskOp::Show { task_id },
        } => ("rsi_task_show", json!({ "task_id": task_id })),
        Cmd::Task {
            op: TaskOp::Delete { task_id },
        } => ("rsi_task_delete", json!({ "task_id": task_id })),
        Cmd::Task {
            op:
                TaskOp::Upsert {
                    task_id,
                    name,
                    description,
                    evaluator_script,
                    evaluator_file,
                },
        } => {
            let evaluator_script = match (evaluator_script, evaluator_file) {
                (_, Some(path)) => std::fs::read_to_string(&path)
                    .map_err(|e| format!("cannot read evaluator file {}: {e}", path.display()))?,
                (Some(s), None) => s,
                (None, None) => {
                    return Err("give --evaluator-file <path> or --evaluator-script <text>".into())
                }
            };
            (
                "rsi_task_upsert",
                json!({
                    "task_id": task_id,
                    "name": name,
                    "description": description,
                    "evaluator_script": evaluator_script,
                }),
            )
        }
        Cmd::Step {
            task_id,
            operator,
            provider,
            sandbox_mode,
            timeout_secs,
        } => (
            "rsi_step",
            json!({
                "task_id": task_id,
                "operator": operator,
                "provider": provider,
                "sandbox_mode": sandbox_mode,
                "timeout_secs": timeout_secs,
            }),
        ),
        Cmd::Campaign {
            task_id,
            steps,
            provider,
            sandbox_mode,
            timeout_secs,
        } => (
            "rsi_campaign",
            json!({
                "task_id": task_id,
                "steps": steps,
                "provider": provider,
                "sandbox_mode": sandbox_mode,
                "timeout_secs": timeout_secs,
            }),
        ),
        Cmd::Frontier { task_id, limit } => (
            "rsi_get_frontier",
            json!({ "task_id": task_id, "limit": limit }),
        ),
        Cmd::Cards {
            task_id,
            operator,
            status,
            limit,
        } => (
            "rsi_card_list",
            obj(vec![
                ("task_id", Some(json!(task_id))),
                ("operator", operator.map(Value::String)),
                ("status", status.map(Value::String)),
                ("limit", Some(json!(limit))),
            ]),
        ),
        Cmd::Card { card_id } => ("rsi_card_show", json!({ "card_id": card_id })),
        Cmd::Lineage { task_id } => ("rsi_lineage", json!({ "task_id": task_id })),
        Cmd::Evaluate {
            task_id,
            code,
            operator,
            sandbox_mode,
            timeout_secs,
            no_record,
        } => (
            "rsi_evaluate",
            json!({
                "task_id": task_id,
                "code": code,
                "operator": operator,
                "sandbox_mode": sandbox_mode,
                "timeout_secs": timeout_secs,
                "record": !no_record,
            }),
        ),
        Cmd::DemoSeed => ("rsi_demo_seed", json!({})),
        Cmd::Diff { card_a, card_b } => (
            "rsi_diff",
            json!({
                "card_a_id": card_a,
                "card_b_id": card_b,
            }),
        ),
        Cmd::Probe { url } => ("rsi_llm_probe", obj(vec![("url", url.map(Value::String))])),
        Cmd::Hyperparams {
            w_score,
            w_delta,
            w_novelty,
        } => {
            if w_score.is_none() && w_delta.is_none() && w_novelty.is_none() {
                ("rsi_hyperparams_get", json!({}))
            } else {
                (
                    "rsi_hyperparams_set",
                    obj(vec![
                        ("w_score", w_score.map(|v| json!(v))),
                        ("w_delta", w_delta.map(|v| json!(v))),
                        ("w_novelty", w_novelty.map(|v| json!(v))),
                    ]),
                )
            }
        }
        Cmd::Prune {
            task_id,
            keep_frontier,
        } => (
            "rsi_cards_prune",
            json!({
                "task_id": task_id,
                "keep_frontier": keep_frontier,
            }),
        ),
        Cmd::Export { task_id, .. } => (
            "rsi_export",
            json!({
                "task_id": task_id,
            }),
        ),
        Cmd::Import { file } => {
            let content = std::fs::read_to_string(&file)
                .map_err(|e| format!("failed to read bundle file '{}': {e}", file.display()))?;
            let val: Value = serde_json::from_str(&content)
                .map_err(|e| format!("invalid JSON in bundle file '{}': {e}", file.display()))?;
            ("rsi_import", json!({ "bundle": val }))
        }
        Cmd::DatasetExport {
            task_id,
            format,
            min_delta,
            ..
        } => (
            "rsi_dataset_export",
            json!({
                "task_id": task_id,
                "format": format,
                "min_delta": min_delta,
            }),
        ),
        Cmd::Bandit { task_id } => (
            "rsi_bandit_status",
            json!({
                "task_id": task_id,
            }),
        ),
        Cmd::Train {
            task_id,
            epochs,
            learning_rate,
            dry_run,
        } => (
            "rsi_train_step",
            json!({
                "task_id": task_id,
                "epochs": epochs,
                "learning_rate": learning_rate,
                "dry_run": dry_run,
            }),
        ),
        Cmd::Promote {
            task_id,
            trials,
            steps,
            sandbox_mode,
            timeout_secs,
            baseline,
            candidate,
        } => (
            "rsi_promote_gate",
            obj(vec![
                ("task_id", Some(json!(task_id))),
                ("trials", Some(json!(trials))),
                ("steps", Some(json!(steps))),
                ("sandbox_mode", Some(json!(sandbox_mode))),
                ("timeout_secs", Some(json!(timeout_secs))),
                ("baseline_alias", baseline.map(Value::String)),
                ("candidate_alias", candidate.map(Value::String)),
            ]),
        ),
        Cmd::HelpTabs { tab, search } => (
            "rsi_help",
            obj(vec![
                ("tab", Some(json!(tab))),
                ("search", search.map(Value::String)),
            ]),
        ),
        Cmd::Tools | Cmd::Mcp => return Err("not a tool verb".into()),
    })
}

struct Host {
    ctx: Ctx,
}

impl ToolHost for Host {
    fn server_name(&self) -> &str {
        APP
    }
    fn server_version(&self) -> &str {
        VERSION
    }
    fn tools(&self) -> Vec<Value> {
        tools()
    }
    fn call(&mut self, name: &str, args: &Value) -> Result<Value, String> {
        let cmd = Command::from_tool(name, args)?;
        dispatch(&mut self.ctx, cmd)
    }
    fn resources(&self) -> Vec<Value> {
        rsicraft_core::skills::bundled_skills()
            .iter()
            .map(|s| {
                json!({
                    "uri": s.uri,
                    "name": s.name,
                    "title": s.title,
                    "description": s.description,
                    "mimeType": "text/markdown",
                })
            })
            .collect()
    }
    fn read_resource(&self, uri: &str) -> Result<Value, String> {
        let skill = rsicraft_core::skills::get_skill(uri)
            .ok_or_else(|| format!("unknown resource '{uri}'"))?;
        Ok(json!({
            "contents": [{
                "uri": skill.uri,
                "mimeType": "text/markdown",
                "text": skill.content,
            }]
        }))
    }
}

fn scalar(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "-".into(),
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if n.is_f64() {
                    return format!("{f:.4}");
                }
            }
            n.to_string()
        }
        other => other.to_string(),
    }
}

const COLUMN_ORDER: &[&str] = &[
    "rank",
    "card_id",
    "task_id",
    "name",
    "operator",
    "status",
    "primary_score",
    "delta_score",
    "novelty_score",
    "composite_utility",
    "children_count",
    "execution_time_ms",
    "parent_a_id",
    "parent_b_id",
];

const HIDDEN_COLS: &[&str] = &["code", "evaluator_script", "raw_output", "code_hash"];

fn columns(rows: &[Value]) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for r in rows.iter().filter_map(Value::as_object) {
        for (k, v) in r {
            if !v.is_object()
                && !v.is_array()
                && !HIDDEN_COLS.contains(&k.as_str())
                && !keys.contains(k)
            {
                keys.push(k.clone());
            }
        }
    }
    keys.sort_by_key(|k| {
        (
            COLUMN_ORDER
                .iter()
                .position(|c| c == k)
                .unwrap_or(usize::MAX),
            k.clone(),
        )
    });
    keys
}

fn render(v: &Value) -> String {
    const LIST_KEYS: [&str; 4] = ["frontier", "cards", "tasks", "steps"];
    let Some(obj) = v.as_object() else {
        return scalar(v);
    };
    if obj.contains_key("configured_model") && obj.contains_key("ok") {
        return render_probe(obj);
    }
    if let Some(diff) = obj.get("diff_text").and_then(Value::as_str) {
        return diff.to_string();
    }
    if let Some(tabs) = obj.get("tabs").and_then(Value::as_array) {
        let mut out = String::new();
        out.push_str("=== rsicraft help ===\n");
        if let Some(index) = obj.get("tabs_available").and_then(Value::as_array) {
            let ids: Vec<&str> = index
                .iter()
                .filter_map(|t| t.get("id").and_then(Value::as_str))
                .collect();
            out.push_str(&format!(
                "Tabs: {} (rsicraft help-tabs --tab <id>, or --search <word>)\n\n",
                ids.join(", ")
            ));
        }
        for t in tabs {
            let id = t.get("id").and_then(Value::as_str).unwrap_or("");
            let title = t.get("title").and_then(Value::as_str).unwrap_or("");
            let badge = t.get("badge").and_then(Value::as_str).unwrap_or("");
            let summary = t.get("summary").and_then(Value::as_str).unwrap_or("");
            out.push_str(&format!("## {title} [{id} | {badge}]\n{summary}\n\n"));
            if let Some(sections) = t.get("sections").and_then(Value::as_array) {
                for s in sections {
                    let h = s.get("heading").and_then(Value::as_str).unwrap_or("");
                    let b = s.get("body").and_then(Value::as_str).unwrap_or("");
                    out.push_str(&format!("  ### {h}\n{}\n", wrap_indent(b, 2, 88)));
                    if let Some(code) = s.get("code_example").and_then(Value::as_str) {
                        out.push_str(&format!(
                            "  Example:\n    {}\n",
                            code.replace('\n', "\n    ")
                        ));
                    }
                    out.push('\n');
                }
            }
        }
        return out;
    }
    let Some(list_key) = LIST_KEYS
        .iter()
        .find(|k| obj.get(**k).is_some_and(Value::is_array))
    else {
        return format!(
            "{}\n",
            serde_json::to_string_pretty(v).unwrap_or_else(|e| e.to_string())
        );
    };
    let mut out = String::new();
    for (k, val) in obj {
        if !val.is_array() && !val.is_object() {
            out.push_str(&format!("{k}: {}\n", scalar(val)));
        }
    }
    let rows = obj
        .get(*list_key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if rows.is_empty() {
        out.push_str("(none)\n");
        return out;
    }
    let cols = columns(&rows);
    let table: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            cols.iter()
                .map(|c| r.get(c).map(scalar).unwrap_or_else(|| "-".into()))
                .collect()
        })
        .collect();
    let widths: Vec<usize> = cols
        .iter()
        .enumerate()
        .map(|(i, c)| {
            table
                .iter()
                .map(|r| r.get(i).map_or(0, |s| s.chars().count()))
                .chain([c.chars().count()])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let line = |cells: &[String]| -> String {
        let padded: Vec<String> = cells
            .iter()
            .zip(&widths)
            .map(|(c, w)| format!("{c}{}", " ".repeat(w.saturating_sub(c.chars().count()))))
            .collect();
        format!("{}\n", padded.join("  ").trim_end())
    };
    out.push_str(&line(&cols));
    for r in &table {
        out.push_str(&line(r));
    }
    out
}

fn run_cli_smoke(db: Option<PathBuf>) -> Result<(), String> {
    let reg = match db {
        Some(path) => Registry::open(&path).map_err(|e| e.to_string())?,
        None => Registry::open_in_memory().map_err(|e| e.to_string())?,
    };
    let mut ctx = Ctx::new(reg, None, None, None);
    dispatch(&mut ctx, Command::Status)?;
    dispatch(&mut ctx, Command::HardwareProfile)?;
    dispatch(
        &mut ctx,
        Command::Help {
            tab: Some("api".into()),
            search: None,
        },
    )?;
    let n = tools().len();
    if n != 27 {
        return Err(format!("expected 27 tools, found {n}"));
    }
    println!("smoke: ok (rsicraft {VERSION}, tools=27)");
    Ok(())
}

/// Print what went wrong in a result that is otherwise a success (skipped
/// campaign steps, an aborted campaign, a failed provider check) to stderr,
/// so it is never silent. Exit code 2 means "ran, but stopped early or found
/// a problem"; stdout stays parseable either way.
fn report_problems(v: &Value) -> ExitCode {
    let mut code = ExitCode::SUCCESS;
    if let Some(warnings) = v
        .get("provider")
        .and_then(|p| p.get("warnings"))
        .and_then(Value::as_array)
    {
        for w in warnings.iter().filter_map(Value::as_str) {
            eprintln!("rsicraft: warning: {w}");
        }
    }
    if let Some(skipped) = v.get("skipped").and_then(Value::as_array) {
        if !skipped.is_empty() {
            eprintln!("rsicraft: {} step(s) produced no card:", skipped.len());
            for s in skipped {
                eprintln!(
                    "  step {} {} [{}]: {}",
                    s.get("step").and_then(Value::as_u64).unwrap_or(0),
                    s.get("operator").and_then(Value::as_str).unwrap_or("?"),
                    s.get("kind").and_then(Value::as_str).unwrap_or("other"),
                    s.get("error").and_then(Value::as_str).unwrap_or("")
                );
            }
        }
    }
    if v.get("aborted").and_then(Value::as_bool) == Some(true) {
        eprintln!(
            "rsicraft: campaign stopped early: {}",
            v.get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown reason")
        );
        code = ExitCode::from(2);
    }
    if v.get("configured_model").is_some() && v.get("ok").and_then(Value::as_bool) == Some(false) {
        code = ExitCode::from(2);
    }
    code
}

/// Human-readable form of an `rsi_llm_probe` report.
/// Word-wrap `text` to `width` columns with `indent` leading spaces, keeping
/// its own line breaks. List items get a hanging indent.
fn wrap_indent(text: &str, indent: usize, width: usize) -> String {
    let pad = " ".repeat(indent);
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            out.push(String::new());
            continue;
        }
        let hang = if line.starts_with("- ") {
            2
        } else {
            match line.split_once(". ") {
                Some((n, _)) if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => {
                    n.len() + 2
                }
                _ => 0,
            }
        };
        let mut cur = pad.clone();
        for word in line.split_whitespace() {
            let fresh = cur.trim().is_empty();
            if !fresh && cur.len() + 1 + word.len() > width {
                out.push(cur);
                cur = format!("{pad}{}", " ".repeat(hang));
            } else if !fresh {
                cur.push(' ');
            }
            cur.push_str(word);
        }
        out.push(cur);
    }
    out.join("\n")
}

fn render_probe(obj: &Map<String, Value>) -> String {
    let s = |k: &str| obj.get(k).map(scalar).unwrap_or_else(|| "-".into());
    let mut out = String::new();
    let ok = obj.get("ok").and_then(Value::as_bool) == Some(true);
    out.push_str(&format!(
        "LLM provider check: {}\n",
        if ok { "OK" } else { "PROBLEM" }
    ));
    out.push_str(&format!("  address        {}\n", s("base_url")));
    out.push_str(&format!("  server         {}\n", s("provider")));
    out.push_str(&format!(
        "  model          {} (served: {})\n",
        s("configured_model"),
        s("model_available")
    ));
    out.push_str(&format!(
        "  budget         {} tokens, {}s timeout, API key {}\n",
        s("max_tokens"),
        s("timeout_secs"),
        if obj.get("api_key_set").and_then(Value::as_bool) == Some(true) {
            "set"
        } else {
            "not set"
        }
    ));
    if let Some(models) = obj.get("available_models").and_then(Value::as_array) {
        if !models.is_empty() {
            let names: Vec<&str> = models.iter().filter_map(Value::as_str).take(20).collect();
            out.push_str(&format!("  served models  {}\n", names.join(", ")));
        }
    }
    if let Some(f) = obj.get("failure").filter(|f| !f.is_null()) {
        out.push_str(&format!(
            "\nProblem [{}]: {}\nFix: {}\n",
            f.get("kind").and_then(Value::as_str).unwrap_or("?"),
            f.get("message").and_then(Value::as_str).unwrap_or(""),
            f.get("remedy").and_then(Value::as_str).unwrap_or("")
        ));
    }
    if let Some(ws) = obj.get("warnings").and_then(Value::as_array) {
        for w in ws.iter().filter_map(Value::as_str) {
            out.push_str(&format!("\nWarning: {w}\n"));
        }
    }
    out
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if cli.smoke {
        return match run_cli_smoke(cli.db.clone()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("smoke: failed: {e}");
                ExitCode::FAILURE
            }
        };
    }
    let Some(cmd) = cli.cmd else {
        eprintln!("rsicraft: a subcommand is required. Pass --smoke for the headless check.");
        return ExitCode::FAILURE;
    };
    if matches!(&cmd, Cmd::Tools) {
        println!(
            "{}",
            serde_json::to_string_pretty(&tools()).unwrap_or_default()
        );
        return ExitCode::SUCCESS;
    }
    let db_path = cli.db.clone().unwrap_or_else(default_db);
    let reg = match Registry::open(&db_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("rsicraft: cannot open {}: {e}", db_path.display());
            return ExitCode::FAILURE;
        }
    };
    let mut ctx = Ctx::new(
        reg,
        cli.llama_url.as_deref(),
        cli.model.as_deref(),
        cli.docker_image.as_deref(),
    );
    if matches!(&cmd, Cmd::Mcp) {
        eprintln!("[rsicraft] db {}", db_path.display());
        let mut host = Host { ctx };
        return match serve_stdio(&mut host) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("rsicraft mcp: {e}");
                ExitCode::FAILURE
            }
        };
    }

    let export_path = if let Cmd::Export { out: Some(p), .. } = &cmd {
        Some(p.clone())
    } else {
        None
    };
    let json_out = cli.json;
    let res = to_tool(cmd)
        .and_then(|(name, args)| Command::from_tool(name, &args))
        .and_then(|cmd| dispatch(&mut ctx, cmd));

    match res {
        Ok(v) => {
            if let Some(p) = export_path {
                let s = serde_json::to_string_pretty(&v).unwrap_or_default();
                if let Err(e) = std::fs::write(&p, s) {
                    eprintln!("rsicraft export: failed to write to {}: {e}", p.display());
                    return ExitCode::FAILURE;
                }
                println!("Exported task bundle to {}", p.display());
            } else if json_out {
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            } else {
                print!("{}", render(&v));
            }
            report_problems(&v)
        }
        Err(e) => {
            if json_out {
                println!("{}", json!({ "error": e }));
            } else {
                eprintln!("rsicraft: {e}");
            }
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp::handle_line;

    fn test_host() -> Host {
        let reg = Registry::open_in_memory().unwrap();
        Host {
            ctx: Ctx::new(reg, None, None, None),
        }
    }

    #[test]
    fn mcp_probe_initialize_list_and_call() {
        let mut h = test_host();
        let init = handle_line(
            &mut h,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
        )
        .unwrap();
        assert_eq!(init["result"]["serverInfo"]["name"], "rsicraft");

        assert!(handle_line(
            &mut h,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#
        )
        .is_none());

        let list =
            handle_line(&mut h, r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).unwrap();
        assert_eq!(list["result"]["tools"].as_array().map(Vec::len), Some(27));

        let res_list = handle_line(
            &mut h,
            r#"{"jsonrpc":"2.0","id":21,"method":"resources/list"}"#,
        )
        .unwrap();
        let resources = res_list["result"]["resources"].as_array().unwrap();
        assert_eq!(resources.len(), 2);

        let res_read = handle_line(
            &mut h,
            r#"{"jsonrpc":"2.0","id":22,"method":"resources/read","params":{"uri":"skill://rsicraft-workflow"}}"#,
        )
        .unwrap();
        let res_text = res_read["result"]["contents"][0]["text"].as_str().unwrap();
        assert!(res_text.contains("rsi_llm_probe"));

        let skill_call = handle_line(
            &mut h,
            r#"{"jsonrpc":"2.0","id":23,"method":"tools/call","params":{"name":"rsicraft_skill_read","arguments":{"name":"rsicraft-ergonomics"}}}"#,
        )
        .unwrap();
        assert_eq!(skill_call["result"]["isError"], false, "{skill_call}");
        let skill_text = skill_call["result"]["content"][0]["text"].as_str().unwrap();
        assert!(skill_text.contains("rsi_hyperparams_set"));

        let seed = handle_line(
            &mut h,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"rsi_demo_seed","arguments":{}}}"#,
        )
        .unwrap();
        assert_eq!(seed["result"]["isError"], false, "{seed}");

        let frontier = handle_line(
            &mut h,
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"rsi.get_frontier","arguments":{"task_id":"opt-synth"}}}"#,
        )
        .unwrap();
        assert_eq!(frontier["result"]["isError"], false, "{frontier}");
        let text = frontier["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"opt-synth\""));
    }

    #[test]
    fn cli_verbs_map_to_known_tools() {
        let verbs = vec![
            Cmd::Status,
            Cmd::Hardware,
            Cmd::Task { op: TaskOp::List },
            Cmd::Step {
                task_id: "opt-synth".into(),
                operator: "DRAFT".into(),
                provider: "demo".into(),
                sandbox_mode: "demo".into(),
                timeout_secs: 30,
            },
            Cmd::Campaign {
                task_id: "opt-synth".into(),
                steps: 4,
                provider: "demo".into(),
                sandbox_mode: "demo".into(),
                timeout_secs: 30,
            },
            Cmd::Frontier {
                task_id: "opt-synth".into(),
                limit: 10,
            },
            Cmd::Cards {
                task_id: "opt-synth".into(),
                operator: None,
                status: None,
                limit: 20,
            },
            Cmd::Lineage {
                task_id: "opt-synth".into(),
            },
            Cmd::DemoSeed,
            Cmd::Diff {
                card_a: "c-1".into(),
                card_b: "c-2".into(),
            },
            Cmd::Probe { url: None },
            Cmd::Hyperparams {
                w_score: None,
                w_delta: None,
                w_novelty: None,
            },
            Cmd::Task {
                op: TaskOp::Delete {
                    task_id: "test".into(),
                },
            },
            Cmd::Prune {
                task_id: "opt-synth".into(),
                keep_frontier: 10,
            },
            Cmd::Export {
                task_id: "opt-synth".into(),
                out: None,
            },
            Cmd::HelpTabs {
                tab: "all".into(),
                search: None,
            },
            Cmd::DatasetExport {
                task_id: "opt-synth".into(),
                format: "all".into(),
                min_delta: 0.0,
                out: None,
            },
            Cmd::Bandit {
                task_id: "opt-synth".into(),
            },
            Cmd::Train {
                task_id: "opt-synth".into(),
                epochs: 3,
                learning_rate: 0.0002,
                dry_run: true,
            },
            Cmd::Promote {
                task_id: "opt-synth".into(),
                trials: 5,
                steps: 6,
                sandbox_mode: "local".into(),
                timeout_secs: 30,
                baseline: None,
                candidate: None,
            },
        ];
        let known: Vec<String> = tools()
            .iter()
            .filter_map(|t| t["name"].as_str().map(str::to_string))
            .collect();
        for v in verbs {
            let (name, _) = to_tool(v).unwrap();
            assert!(known.iter().any(|k| k == name), "missing tool {name}");
        }
    }

    #[test]
    fn cli_smoke_checks_status_hardware_help_and_tool_count() {
        assert!(run_cli_smoke(None).is_ok());
    }

    #[test]
    fn promote_cli_forwards_steps_sandbox_and_aliases() {
        let mapped = to_tool(Cmd::Promote {
            task_id: "opt-synth".into(),
            trials: 5,
            steps: 6,
            sandbox_mode: "docker".into(),
            timeout_secs: 45,
            baseline: Some("rsi-base-35b".into()),
            candidate: Some("rsi-frontis-35b".into()),
        });
        assert!(mapped.is_ok());
        if let Ok((name, args)) = mapped {
            assert_eq!(name, "rsi_promote_gate");
            assert_eq!(args["steps"], json!(6));
            assert_eq!(args["sandbox_mode"], json!("docker"));
            assert_eq!(args["timeout_secs"], json!(45));
            assert_eq!(args["baseline_alias"], json!("rsi-base-35b"));
            assert_eq!(args["candidate_alias"], json!("rsi-frontis-35b"));
        }
    }
}
