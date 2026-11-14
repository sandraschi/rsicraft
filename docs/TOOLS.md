# Complete 26-Tool Reference: RSICraft (`MCP 2024-11-05` & CLI)

**Navigation:** [README](../README.md) · [Architecture](ARCHITECTURE.md) · [Configuration](CONFIGURATION.md) · [9-Tab Help Manual](../HELP.md) · [RSI Limits & Benchmarking](RSI_ENGINEERING_LIMITS_AND_BENCHMARKING.md) · [Bostrom & Yudkowsky Analysis](BOSTROM_YUDKOWSKY_AND_RSI_REALITY.md)

Every tool is exposed over stdio MCP (`rsicraft mcp`, accepting both `rsi_*` and dotted `rsi.*` names) and as a 1:1 matching CLI subcommand (`rsicraft <verb> [--json]`).

---

## 1. System, Hardware, Probe & Multitab Help Tools

### `rsi_status` (CLI: `rsicraft status`)
Returns SQLite `WAL` store metrics (`tasks_count`, `cards_count`, `success_cards_count`, `frontier_entries_count`, `unique_code_hashes`) and active RTX 4090 inference configuration.

### `rsi_hardware_profile` (CLI: `rsicraft hardware`)
Returns the Goliath hardware specification (Ryzen 9 7900X, 64 GB DDR5, RTX 4090 24 GB VRAM), `Frontis-MA1` 30B/35B MoE VRAM budget (`18.0–19.5 GB`), and the recommended `llama-server` launch command.

### `rsi_llm_probe` (CLI: `rsicraft probe [--url <url>]`)
Actively pings the configured LLM endpoint (`/v1/models`), returning latency in ms, reachability status, and list of available models.
- **`url`**: Optional custom `llama-server` URL to probe (defaults to configured `base_url`).

### `rsi_help` (CLI: `rsicraft help-tabs [--tab <slug>] [--search <query>]`)
Queries the 9-tab Operator Manual, RSI Geopolitics Dossier, Tsinghua/Frontis.AI OpenRSI Dossier, and Competitive Matrix.
- **`tab`**: `"all"` (default), `"overview"`, `"rsi_landscape"`, `"tsinghua_openrsi"`, `"wrappee"`, `"operators"`, `"compare"`, `"api"`, `"troubleshoot"`, `"faq"`.
- **`search`**: Optional case-insensitive substring filter across all tabs and sections.

---

## 2. Task Management, Export/Import & Delete Tools

### `rsi_task_upsert` (CLI: `rsicraft task upsert`)
Creates or updates an optimization task and its sandbox evaluator script.
- **Required arguments:** `task_id`, `name`, `description`, `evaluator_script` (must print `RSI_METRIC_JSON={"status":"SUCCESS","score":<float>}`).

### `rsi_task_list` (CLI: `rsicraft task list`)
Lists all registered evolutionary optimization tasks.

### `rsi_task_show` (CLI: `rsicraft task show <task_id>`)
Returns task metadata, dynamic score/delta normalization bounds (`TaskScoreBounds`), and the current top-K materialized frontier.

### `rsi_task_delete` (CLI: `rsicraft task delete <task_id>`)
Permanently deletes a task and cascade deletes all its associated experience cards and frontier records without leaving dangling records.

### `rsi_export` (CLI: `rsicraft export --task-id <id> [--out <path>]`)
Exports a complete task lineage bundle (task definition, decompressed CAS cards, and Pareto frontier) to a standalone JSON document.

### `rsi_import` (CLI: `rsicraft import --file <path>`)
Imports a complete task lineage bundle from JSON into SQLite, re-registering the task, experience cards, and frontier.

---

## 3. Evolutionary Execution & Multi-Language Sandbox Tools

### `rsi_step` (CLI: `rsicraft step`)
Executes a single evolutionary step:
1. Samples 0, 1, or 2 parents from the SQLite `WAL` store using composite utility $U_i$.
2. Builds operator-specific prompts (`DRAFT` `0.70`, `IMPROVE` `0.40`, `DEBUG` `0.20`, `CROSSOVER` `0.50`).
3. Queries `llama-server` (`provider = "llama"`) or the deterministic oracle (`provider = "demo"`).
4. Evaluates synthesized code in `sandbox_mode` (`"docker"`, `"local"`, or `"demo"`).
5. Compresses code via BLAKE3 + Zstd, records the experience card, recalculates $U_i$ across the task, and refreshes `task_frontiers`.

### `rsi_campaign` (CLI: `rsicraft campaign`)
Runs `steps` (`1..=64`) autonomous evolutionary steps cycling across `DRAFT -> IMPROVE -> DEBUG -> CROSSOVER` and returns the full trajectory and best score.

### `rsi_evaluate` (CLI: `rsicraft evaluate`)
Directly benchmarks a candidate snippet against a task's evaluator in the sandbox (`record = true` by default). Automatically supports both **Python 3.12** and **native Rust compilation** (`rustc` compiling `candidate.rs` + `evaluator.rs`).

### `rsi_demo_seed` (CLI: `rsicraft demo-seed`)
Seeds an empty database with the `opt-synth` (Vector Max) benchmark task and a 4-card evolutionary progression (`DRAFT -> DEBUG -> IMPROVE -> CROSSOVER`).

---

## 4. Frontier, Lineage DAG, Diff & Prune Tools

### `rsi_get_frontier` (CLI: `rsicraft frontier [--task-id <id>] [--limit <k>]`)
Returns the materialized top-K Pareto frontier experience cards for a task.

### `rsi_card_list` (CLI: `rsicraft cards [--task-id <id>] [--operator <op>] [--status <st>]`)
Lists experience cards ordered by composite utility $U_i$, with optional operator and execution-status filters.

### `rsi_card_show` (CLI: `rsicraft card <card_id>`)
Decompresses the Zstd payload, verifies the BLAKE3 hash, and returns the full experience card including source code and sandbox stdout/stderr.

### `rsi_lineage` (CLI: `rsicraft lineage [--task-id <id>]`)
Exports the complete evolutionary lineage DAG (`nodes` and `parent_a`/`parent_b` `edges`) for graph visualization.

### `rsi_diff` (CLI: `rsicraft diff --card-a <id> --card-b <id>`)
Generates a line-by-line unified text diff comparing decompressed source codes between two experience cards (e.g. parent vs child candidate).

### `rsi_cards_prune` (CLI: `rsicraft prune --task-id <id> [--keep-frontier <k>]`)
Prunes low-utility non-frontier cards for a task, keeping at least `keep_frontier` top Pareto cards while maintaining database cleaniness.

---

## 5. Runtime Hyperparameter Tuning Tools

### `rsi_hyperparams_get` (CLI: `rsicraft hyperparams`)
Returns current composite utility weights ($\lambda_s, \lambda_\delta, \lambda_\nu$) used for stochastic parent selection.

### `rsi_hyperparams_set` (CLI: `rsicraft hyperparams [--w-score <s] [--w-delta <d>] [--w-novelty <n>]`)
Dynamically adjusts composite utility weights ($\lambda_s, \lambda_\delta, \lambda_\nu$) at runtime without restarting the daemon.

---

## 6. Frontis Ladder Parity Tools (Rungs 2, 3, 4: Trace Curation, Bandit & Closed-Loop RSI)

### `rsi_dataset_export` (CLI: `rsicraft dataset-export [--task-id <id>] [--format <all|sft|dpo>] [--min-delta <d>]`)
Rung 2 Self-Evolution: Exports curated SFT trajectories (positive deltas, compiler/runtime error repairs) and DPO contrastive preference pairs from the SQLite WAL DAG ready for fine-tuning.

### `rsi_bandit_status` (CLI: `rsicraft bandit [--task-id <id>]`)
Advisory per-operator statistics (pulls, wins, Beta posterior parameters, mean reward) and a recommended next operator. Not Thompson sampling yet and not used by campaigns.

### `rsi_train_step` (CLI: `rsicraft train [--task-id <id>] [--epochs <n>] [--learning-rate <lr>] [--dry-run]`)
Exports the curated SFT traces of a task as chat JSONL and runs `sandboxes/train_lora.py` on them (needs `RSICRAFT_TRAIN_BASE_MODEL` and a CUDA Python environment). Produces a candidate adapter; never changes the active model. Trains or fails; `dry_run` changes nothing. The harness has not been run end to end yet.

### `rsi_promote_gate` (CLI: `rsicraft promote --task-id <id> --candidate <alias> [--baseline <alias>] [--trials <n>] [--steps <n>] [--sandbox-mode <local|docker>] [--timeout-secs <n>]`)
A/B gate between two model aliases the endpoint already serves. CLI `--candidate` and `--baseline` map to MCP `candidate_alias` and `baseline_alias`. `trials` is clamped to 3..=20 (default 5). `steps` is clamped to 1..=64 (default 6). `sandbox_mode` defaults to `local`. `timeout_secs` defaults to 30. Runs that many fresh searches per model on scratch copies of the task, interleaved. Promotes the candidate alias on the task (and increments `active_model_gen`) only if Welch $t \ge 2$. `candidate_alias` is required. The demo sandbox is rejected. The gate does not load an adapter.
