# RSICraft Help & Operator Manual

RSICraft is a local-first **Recursive Self-Improvement (RSI) & evolutionary program search studio** implemented in 100% safe Rust (`rsicraft-core`, `rsicraft-mcp`, `rsicraft-app`). It lets a local or OpenAI-compatible language model write, evaluate, repair, and recombine candidate programs in a closed loop, persisting every attempt with its full lineage in a SQLite WAL database backed by a BLAKE3 + Zstd Content-Addressable Store (CAS).

> **Note on Built-in Help:** The in-app Help Center (`Help -> Project Files & Docs...`), `rsicraft help-tabs`, and the `rsi_help` MCP tool all read the 10-tab structured manual in `crates/rsicraft-core/src/help.rs`. This file (`HELP.md`) is the complete standalone manual and ecosystem reference.

---

## 1. What RSICraft Is & Does + Get Started

### What RSICraft Is & Does

You give RSICraft a **task** (a natural-language specification such as `"write solve(n) returning the number of primes below n, as fast as possible"`) and an **evaluator** (a Python script or Rust harness that executes a candidate and prints a single `RSI_METRIC_JSON={"status":"SUCCESS","score":...}` line).

RSICraft drives an evolutionary search loop across four operators:

- **DRAFT**: Synthesizes a fresh candidate solution from the task description.
- **IMPROVE**: Mutates a high-utility working parent to optimize score or runtime.
- **DEBUG**: Repairs a failed parent (`COMPILE_ERROR`, `RUNTIME_ERROR`, `TIMEOUT`, `METRIC_PARSE_ERROR`) using the tail of its sandbox execution log.
- **CROSSOVER**: Combines ideas from two distinct high-utility parents into a stronger child.

**What it is not:** During a search (`rsi_step` / `rsi_campaign`), the LLM weights do not change — only the archive of experience cards grows (**Rung 1**). **Rung 2** tools (`rsi_dataset_export`, `rsi_train_step`, `rsi_promote_gate`) exist in the repository to export SFT/DPO traces, train a LoRA adapter, and run an A/B promotion gate, but model weight improvement has not yet been demonstrated in measured runs.

### Get Started (5-Step First-Run Tour)

1. **Build the workspace:**
   ```powershell
   cargo build --release
   ```
2. **Try demo mode (no GPU or LLM server required):**
   ```powershell
   .\target\release\rsicraft --db demo.db demo-seed
   .\target\release\rsicraft --db demo.db campaign --task-id opt-synth --steps 4 --provider demo --sandbox-mode demo
   .\target\release\rsicraft --db demo.db lineage --task-id opt-synth
   ```
3. **Connect a local LLM and verify it with `probe`:**
   ```powershell
   $env:RSICRAFT_LLM_URL = 'http://127.0.0.1:11434'   # Ollama (:11434), LM Studio (:1234), llama-server (:8080/8081)
   $env:RSICRAFT_MODEL   = 'qwen2.5-coder:7b'
   .\target\release\rsicraft probe
   ```
   Confirm `probe` reports `OK` and check any context-window warnings before starting a campaign.
4. **Register a real task and run a search:**
   ```powershell
   .\target\release\rsicraft task upsert --task-id primes --name "Prime count" `
     --description "Write solve(n) returning the number of primes below n, as fast as possible. Standard library only." `
     --evaluator-file tasks\pack-v1\primes.py
   .\target\release\rsicraft campaign --task-id primes --steps 8 --provider llama --sandbox-mode local
   .\target\release\rsicraft frontier --task-id primes
   ```
5. **Open the Desktop Studio or inspect diffs:**
   ```powershell
   .\target\release\rsicraft-app
   ```
   Watch the live Bezier Lineage DAG, inspect decompressed source code and unified diffs between parents and children, or open the AI Chat Dock (`Ctrl+J`).

---

## 2. Functions

### Evolutionary Search Operators (`operators.rs`)

Each operator builds a tailored prompt and temperature profile (`DRAFT`, `IMPROVE`, `DEBUG`, `CROSSOVER`) for Python or Rust tasks. Code blocks are extracted from fenced or unfenced model responses, with reasoning-model salvage when a model spends tokens in thought preambles.

### Composite-Utility Parent Selection (`store/utility.rs`)

Parents are sampled stochastically using a composite utility score combining three normalized terms:

- `w_score` — Normalized primary task score from the evaluator.
- `w_delta` — Improvement delta over the parent's score.
- `w_novelty` — Exploration bonus inversely proportional to the parent's child count (`1 / (1 + children)`).

Inspect or tune these weights at any time with `rsi_hyperparams_get` / `rsi_hyperparams_set` (every task is re-ranked immediately upon update).

### Content-Addressable Store & Lineage DAG (`store/cas.rs`, `store/mod.rs`)

Every candidate program is hashed with **BLAKE3**, compressed with **Zstd**, and stored in SQLite WAL (`%LOCALAPPDATA%\rsicraft\rsicraft.db` by default, or `--db` / `RSICRAFT_DB`). Parent-child edges form a directed acyclic graph (`rsi_lineage`), supporting unified diffs (`rsi_diff`), lossless JSON bundle export/import (`rsi_export`, `rsi_import`), and non-frontier pruning (`rsi_cards_prune`).

### Sandboxed Evaluation (`sandbox.rs`)

Three execution backends share the same evaluator contract:

- `demo` — Deterministic offline oracle for testing and UI exploration.
- `local` — Runs Python or `rustc` directly on the host with a configurable timeout (`--timeout-secs`).
- `docker` — Runs inside an isolated container (`rsicraft-evaluator:latest`) with no network, read-only root, dropped privileges, and memory/CPU caps.

### LLM Client & Failure Classification (`llm.rs`)

Talks to any OpenAI-compatible `/v1/chat/completions` and `/v1/models` endpoint (Ollama, LM Studio, `llama-server`, vLLM). Every failure is classified into one of 12 structured `[llm:<class>]` errors (`config`, `unreachable`, `timeout`, `model_not_found`, `auth_required`, `context_overflow`, `server_error`, `bad_request`, `bad_response`, `budget_exhausted`, `empty_answer`, `no_code`) with an actionable fix. Setup errors abort campaigns immediately; transient 5xx/network blips retry with bounded backoff.

### Rung 2 Self-Evolution Tools (`store/dataset.rs`, `bandit.rs`, `trainer.rs`)

- `rsi_dataset_export`: Exports curated SFT trajectories (positive-delta improvements and error repairs) and DPO contrastive preference pairs.
- `rsi_bandit_status`: Reports advisory per-operator pulls, wins, Beta posterior parameters, and a recommended next operator.
- `rsi_train_step`: Invokes `sandboxes/train_lora.py` to post-train a candidate LoRA adapter (requires `RSICRAFT_TRAIN_BASE_MODEL` and CUDA; supports `--dry-run`).
- `rsi_promote_gate`: Runs fresh A/B search trials between baseline and candidate model aliases and promotes only on a statistically clear win (Welch's $t \ge 2$).

---

## 3. Usage (GUI, CLI & MCP)

### Desktop Studio (`rsicraft-app`)

- **Top Menu Bar (8-Menu Standard):** `File`, `Edit`, `View`, `Insert`, `Modify`, `Commands`, `Window`, `Help` with `switch_on_hover` navigation.
- **7 Workspace Views:** Evolutionary Studio, Lineage DAG Canvas, Experience Cards & CAS, Task & Sandbox Lab, RTX 4090 & LLM Config, What is RSI?, and Help & Tools.
- **Keyboard Shortcuts:**
  - `Ctrl+K` — Command Palette (fuzzy search over all actions and commands)
  - `F1` — Keyboard Shortcuts cheat sheet
  - `Ctrl+J` — Toggle local-first AI Chat Dock
  - `Ctrl+N` / `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S` — New Task, Open DB, Save, Save As
  - `Ctrl+Z` / `Ctrl+Y` — Undo / Redo store snapshots
  - `Ctrl+E` — Export Lineage JSON
  - `Ctrl+1` / `Ctrl+MouseWheel` — Reset Zoom / Zoom UI

### Command-Line Interface (`rsicraft`)

Every CLI verb routes through the exact same `Command::from_tool` + `dispatch` pipeline as the MCP server and accepts `--json`:

```powershell
rsicraft --json status
rsicraft --json task list
rsicraft --json step --task-id primes --operator IMPROVE --provider llama --sandbox-mode local
rsicraft --json diff --card-a <id_a> --card-b <id_b>
rsicraft --json export --task-id primes --out primes_lineage.json
rsicraft --json dataset-export --task-id primes --format all --min-delta 0.05
rsicraft help-tabs --tab troubleshoot
rsicraft --smoke
```

### MCP Server (`rsicraft mcp`)

Starts a pure-Rust MCP `2024-11-05` `stdio` server (JSON-RPC on `stdout`, diagnostics on `stderr`). Exposes 27 tools (`rsi_*` or dotted `rsi.*` aliases plus `rsicraft_skill_read`) and MCP `resources/list` + `resources/read` for bundled skills (`skill://rsicraft-workflow`, `skill://rsicraft-ergonomics`).

---

## 4. Tips & Tricks

- **Dry-run before spending GPU time:** Always validate a new task or evaluator with `--provider demo --sandbox-mode demo` or `rsicraft evaluate --no-record` before launching a multi-step LLM campaign.
- **Size `num_ctx` to fit VRAM:** Some local servers default to huge context windows (e.g., 262k tokens on 9B models), causing OOM or HTTP 500 errors. Set `num_ctx` to `8192` or `16384` for single-file algorithm tasks.
- **Auto-discover local LLM servers:** Run `rsicraft probe` with no arguments when a server fails to respond — it scans Ollama (`11434`), LM Studio (`1234`), `llama-server` (`8080`, `8081`), and vLLM (`8000`) and tells you which models are loaded.
- **Escape search plateaus:** If `IMPROVE` keeps refining the same local optimum, raise `w_novelty` via `rsi_hyperparams_set` or run explicit `CROSSOVER` steps to recombine distinct branches of the frontier.
- **Use `--evaluator-file`:** Avoid shell quoting bugs on Windows/PowerShell by keeping evaluators in `.py` files (see `tasks/pack-v1/` and `tasks/pack-v2/`).
- **Keep large stores snappy:** Periodically run `rsi_cards_prune` (keeping `keep_frontier` >= 10) after exporting a backup with `rsi_export`.

---

## 5. Common Pitfalls & Safety

- **Local Sandbox Runs Code As You:** `--sandbox-mode local` executes model-generated Python or Rust directly under your OS user account with full filesystem and network access. **Never** use `local` on untrusted tasks or unreviewed models; build the Docker evaluator image (`sandboxes/`) and pass `--sandbox-mode docker`.
- **Reasoning Token Budget Exhaustion (`budget_exhausted`):** Reasoning models (e.g., Qwen3/DeepSeek-R1 derivatives) can spend the entire `max_tokens` budget thinking without emitting code. Increase `RSICRAFT_MAX_TOKENS` or set `RSICRAFT_REASONING_EFFORT=low`.
- **Expecting Weight Updates from Search:** `rsi_campaign` only grows the SQLite experience-card archive. Model weights never change unless you explicitly run `rsi_train_step` and promote via `rsi_promote_gate`.
- **Cascade Deletion Hazard:** `rsi_task_delete` permanently deletes the task, all its experience cards, and its frontier records. Always run `rsi_export` first if you want to preserve a lineage.
- **Port & URL Mistakes:** Pass the server root (`http://127.0.0.1:11434`), **not** the full `/v1/chat/completions` path, to `--llama-url` or `RSICRAFT_LLM_URL`.

---

## 6. Planned Extensions

Status reflects `docs/FRONTIS_PARITY_PLAN.md`:

- **Rung 2 LoRA Loop Validation:** Complete end-to-end training runs with `rsi_train_step` + `rsi_promote_gate` on consumer 24 GB GPUs and measure whether post-trained adapters beat base models on `pack-v1` / `pack-v2`.
- **Adaptive Bandit Scheduling:** Wire the Beta-posterior Thompson sampling in `bandit.rs` directly into `rsi_campaign` operator selection (currently advisory via `rsi_bandit_status`).
- **Expanded Rust Candidate Synthesis:** Broaden multi-language Rust candidate generation and Cargo-backed evaluator harnesses alongside Python evaluators.

---

## 7. Other Crafts

RSICraft belongs to the **Storytold Craft** pure-Rust desktop & MCP workstation ecosystem:

| Craft App | Domain | Handoffs & Boundaries |
| :--- | :--- | :--- |
| **`rsicraft`** | Recursive Self-Improvement & evolutionary code search | Outputs optimized Python/Rust candidate snippets, lineage JSON bundles, and SFT/DPO datasets (`rsi_dataset_export`). |
| **`codercraft`** | Agentic software engineering & repo workbench | Consumes optimized algorithms or patches produced by `rsicraft` into full repository workflows. |
| **`cadcraft`** | Parametric 2D/3D CAD workstation | Separate geometry/DXF domain; shares the 8-menu Craft bar and `Command::dispatch` architecture. |
| **`celcraft`** | Classical 2D animation & exposure-sheet studio | Separate animation domain; reference implementation for the 8-menu Craft standard. |
| **`filmcraft`** | Non-linear video editing & timeline studio | Separate media/timeline domain; shares the Craft UI and MCP parity contract. |
| **`photocraft`** | Non-destructive image & RAW processing studio | Separate raster/color domain; shares the Craft UI and MCP parity contract. |
| **`deckcraft`** | Presentation & slide authoring workstation | Separate slide/layout domain; shares the Craft UI and MCP parity contract. |

> **Normative Boundary:** Upstream Storytold repositories are never modified from `rsicraft`; all data exchange happens via exported JSON bundles, source files, or MCP tool calls.
