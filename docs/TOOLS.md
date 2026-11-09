# Complete 14-Tool Reference: RSICraft (`MCP 2024-11-05` & CLI)

**Navigation:** [README](../README.md) · [Architecture](ARCHITECTURE.md) · [Configuration](CONFIGURATION.md) · [9-Tab Help Manual](../HELP.md)

Every tool is exposed over stdio MCP (`rsicraft mcp`, accepting both `rsi_*` and dotted `rsi.*` names) and as a 1:1 CLI subcommand (`rsicraft <verb> [--json]`).

---

## 1. System, Hardware & Multitab Help Tools

### `rsi_status` (CLI: `rsicraft status`)
Returns SQLite `WAL` store metrics (`tasks_count`, `cards_count`, `success_cards_count`, `frontier_entries_count`, `unique_code_hashes`) and active RTX 4090 inference configuration.

### `rsi_hardware_profile` (CLI: `rsicraft hardware`)
Returns the Goliath hardware specification (Ryzen 9 7900X, 64 GB DDR5, RTX 4090 24 GB VRAM), `Frontis-MA1` 30B/35B MoE VRAM budget (`18.0–19.5 GB`), and the recommended `llama-server` launch command.

### `rsi_help` (CLI: `rsicraft help-tabs [--tab <slug>] [--search <query>]`)
Queries the 9-tab Operator Manual, RSI Geopolitics Dossier, Tsinghua/Frontis.AI OpenRSI Dossier, and Competitive Matrix.
- **`tab`**: `"all"` (default), `"overview"`, `"rsi_landscape"`, `"tsinghua_openrsi"`, `"wrappee"`, `"operators"`, `"compare"`, `"api"`, `"troubleshoot"`, `"faq"`.
- **`search`**: Optional case-insensitive substring filter across all tabs and sections.

---

## 2. Task Management Tools

### `rsi_task_upsert` (CLI: `rsicraft task upsert`)
Creates or updates an optimization task and its sandbox evaluator script.
- **Required arguments:** `task_id`, `name`, `description`, `evaluator_script` (must print `RSI_METRIC_JSON={"status":"SUCCESS","score":<float>}`).

### `rsi_task_list` (CLI: `rsicraft task list`)
Lists all registered evolutionary optimization tasks.

### `rsi_task_show` (CLI: `rsicraft task show <task_id>`)
Returns task metadata, dynamic score/delta normalization bounds (`TaskScoreBounds`), and the current top-K materialized frontier.

---

## 3. Evolutionary Execution & Sandbox Evaluation Tools

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
Directly benchmarks a candidate Python snippet against a task's evaluator in the sandbox (`record = true` by default).

### `rsi_demo_seed` (CLI: `rsicraft demo-seed`)
Seeds an empty database with the `opt-synth` (Vector Max) benchmark task and a 4-card evolutionary progression (`DRAFT -> DEBUG -> IMPROVE -> CROSSOVER`).

---

## 4. Frontier & Lineage DAG Tools

### `rsi_get_frontier` (CLI: `rsicraft frontier [--task-id <id>] [--limit <k>]`)
Returns the materialized top-K Pareto frontier experience cards for a task.

### `rsi_card_list` (CLI: `rsicraft cards [--task-id <id>] [--operator <op>] [--status <st>]`)
Lists experience cards ordered by composite utility $U_i$, with optional operator and execution-status filters.

### `rsi_card_show` (CLI: `rsicraft card <card_id>`)
Decompresses the Zstd payload, verifies the BLAKE3 hash, and returns the full experience card including Python source code and sandbox stdout/stderr.

### `rsi_lineage` (CLI: `rsicraft lineage [--task-id <id>]`)
Exports the complete evolutionary lineage DAG (`nodes` and `parent_a`/`parent_b` `edges`) for graph visualization.
