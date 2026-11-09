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
    /// llama-server or Ollama base URL (default: http://127.0.0.1:8081).
    #[arg(long, global = true)]
    llama_url: Option<String>,
    /// Model alias passed to /v1/chat/completions (default: frontis).
    #[arg(long, global = true)]
    model: Option<String>,
    /// Docker evaluator image tag (default: rsicraft-evaluator:latest).
    #[arg(long, global = true)]
    docker_image: Option<String>,
    /// Emit machine-readable JSON only.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
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
    /// Actively probe the LLM endpoint (/v1/models) for reachability and latency.
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
    /// Query the 7-tab Operator Manual & Competitive Architecture Matrix (overview, wrappee, operators, compare, api, troubleshoot, faq, all).
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
        #[arg(long)]
        evaluator_script: String,
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
                },
        } => (
            "rsi_task_upsert",
            json!({
                "task_id": task_id,
                "name": name,
                "description": description,
                "evaluator_script": evaluator_script,
            }),
        ),
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
    if let Some(diff) = obj.get("diff_text").and_then(Value::as_str) {
        return diff.to_string();
    }
    if let Some(tabs) = obj.get("tabs").and_then(Value::as_array) {
        let mut out = String::new();
        out.push_str("=== RSICraft 7-Tab Operator Manual & Architecture Guide ===\n");
        out.push_str(
            "Tabs: [overview] [wrappee] [operators] [compare] [api] [troubleshoot] [faq]\n\n",
        );
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
                    out.push_str(&format!("  ### {h}\n  {b}\n"));
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

fn main() -> ExitCode {
    let cli = Cli::parse();
    if matches!(cli.cmd, Cmd::Tools) {
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
    if matches!(cli.cmd, Cmd::Mcp) {
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

    let export_path = if let Cmd::Export {
        out: Some(ref p), ..
    } = cli.cmd
    {
        Some(p.clone())
    } else {
        None
    };
    let json_out = cli.json;
    let res = to_tool(cli.cmd)
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
            ExitCode::SUCCESS
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
        assert_eq!(list["result"]["tools"].as_array().map(Vec::len), Some(22));

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
}
