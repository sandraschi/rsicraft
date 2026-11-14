# Changelog

## Unreleased

- Undo and redo restore SQLite snapshots (up to 8). Open Database and Save As use a file dialog. Save checkpoints a file-backed store. Cut, paste, and select all still have no selection.
- The built-in manual is ten tabs (`start`, `rsi`, `based_on`, `quickstart`, `providers`, `tasks`, `concepts`, `api`, `troubleshoot`, `faq`). The studio Help page and "What is RSI?" page read that text. `docs/GUIDE.md` and `docs/help/index.html` are generated from it.
- LLM failures are classified (`config`, `unreachable`, `timeout`, `model_not_found`, `auth_required`, `context_overflow`, `server_error`, `bad_request`, `bad_response`, `budget_exhausted`, `empty_answer`, `no_code`). A campaign preflights the provider and stops on a fatal class. An answer with no top-level function is `no_code` and is not evaluated. `rsicraft probe` returns that diagnosis and does not fail the call.
- `--llama-url` and `--model` follow `RSICRAFT_LLM_URL` and `RSICRAFT_MODEL`. `RSICRAFT_LLM_API_KEY` is sent as a Bearer token when set.
- Desktop studio gained the 8-menu bar and the five Help dialogs. Engine menu entries call `Command::dispatch`. `switch_on_hover` moves between open menus.
- `rsicraft --smoke` checks status, hardware, help, and the 26-tool count without a subcommand.
- A non-2xx llama-server response now keeps a short response body on the step error.
- `rsi-base-35b` was rewritten so its template and `num_ctx` match `rsi-frontis-35b`. `scripts/align-35b-alias.ps1` is that repair.
- Recorded the finished pack-v2 9B run (`docs/BENCH_AMP_V2.md`): 15 of 15 complete, 2 runs with a success card, amplification gate not met. Handover, PRD goal G2, and the parity plan now say that.
- Corrected documents and in-app help that described a finished 4-rung ladder, Thompson sampling driving campaigns, LoRA advancing the active model, an 8-menu bar, FastMCP, and a 22-tool bus.
- `rsicraft promote` now forwards `--steps`, `--sandbox-mode`, and `--timeout-secs`. Trials passed to `rsi_promote_gate` clamp to 3..=20 (default 5), matching the tool schema.

## 0.3.1 - 2026-11-11

### Documentation Synchronization, Pack-v1 Benchmarks & Recompiled Brochure Suite

- **Documentation Parity:** Synchronized architecture and tools reference with the 3-phase execution pipeline (`plan_step`, `run_step`, `commit_step`) that decouples multi-minute LLM completions and sandbox evaluations from SQLite WAL locks.
- **Desktop Studio Live Run:** Documented the non-blocking background worker thread in `rsicraft-app`, live run dashboard, progress meters, and stop button.
- **Benchmark Suite (`tasks/pack-v1`):** Added documentation for the 6 standard CPU-only algorithmic tasks (`edit_distance`, `inversions`, `lis`, `matmul`, `primes`, `wordfreq`) implementing noise-normalized speedup evaluation against an in-process unoptimized reference algorithm.
- **Model-Swap Harness (`scripts/bench-models.ps1`):** Documented the automated, resumable evaluation runner comparing models on equal task splits.
- **Welch's t-Test Promotion Gate:** Documented the A/B evaluation protocol with interleaved trials across baseline and candidate aliases enforcing $t \ge 2.0$ for promotion.
- **Configuration Reference:** Documented `RSICRAFT_LLM_TIMEOUT_SECS` (default 600s), `RSICRAFT_MAX_TOKENS` (default 12,288), and `RSICRAFT_REASONING_EFFORT`.
- **Recompiled Brochure Suite:** PDF, EPUB, and HTML brochures refreshed and synchronized.

## 0.3.0 - 2026-11-10

### 4-Rung Frontis Ladder & 26-Tool Surface

### Correction and honest rework of the "Frontis ladder" tools (2026-11-10, evening)

An earlier entry in this section claimed full four-rung parity with `arXiv:2607.28568`. It was wrong: the training step printed a hardcoded loss curve without training, the promotion gate awarded every candidate a constant +4.5%, and model generations had no effect on which model was called. Corrected state (details in `docs/FRONTIS_PARITY_PLAN.md` and `HANDOVER.md`):

- **`rsi_train_step`:** exports chat-format JSONL and runs `sandboxes/train_lora.py` (a real PEFT script, not yet run end to end). It trains or fails. A dry run changes nothing. Training never changes the active model.
- **`rsi_promote_gate`:** real A/B between two served model aliases: `trials` fresh searches of `steps` steps per model on scratch copies of the task, promote only at Welch t >= 2. New arguments `steps`, `sandbox_mode`, `timeout_secs`; `candidate_alias` is required; the demo sandbox is refused.
- **Generations are real:** a step sends the task's promoted alias to the endpoint.
- **Step pipeline in three phases** (`plan_step`, `run_step`, `commit_step`, plus `plan_evaluate` / `run_evaluate` / `commit_evaluate` and `campaign_operator`), so the slow phase can run off the store's thread.
- **Desktop app:** steps, campaigns and lab evaluations run on a worker thread. New Live Run panel (running operator, elapsed time, stop-after-this-step, per-step timeline with score bars). The window no longer freezes during LLM calls.
- **Reasoning models:** default completion budget 12,288 tokens; when the answer is empty the last code block drafted in the reasoning field is evaluated instead; code fences nested in markdown lists are dedented; optional `RSICRAFT_REASONING_EFFORT`.
- **Still not true, still to do:** the operator bandit is not Thompson sampling and is not used by campaigns; multi-trial scoring exists in the sandbox but steps use one trial; SFT traces re-synthesize prompts instead of recording the real ones; no held-out task split; no way to serve a trained adapter.

### Earlier additions in this section (tool surface 22 to 26)

- **Multi-trial sandbox evaluation:** `trials` parameter computing mean, sample standard deviation and count across the Docker, local and demo backends.
- **Trace export:** `rsi_dataset_export` / `rsicraft dataset-export` (SFT traces and DPO sibling pairs from the lineage).
- **Operator statistics:** `rsi_bandit_status` / `rsicraft bandit` (advisory).- **Expanded Tool Surface (22 to 26 Tools):** 100% CLI and MCP parity for all 4 new tools across `rsicraft-core`, `rsicraft-mcp`, and `rsicraft-app`.

### Correctness, Sandbox Hardening & Performance Pass

- **Store:** every multi-statement mutation (`record_experience_card`, prune, delete, import, frontier refresh) now runs in one transaction. Utility recompute reads numeric columns only instead of decompressing every card's code on every insert, and no longer stops at 10,000 cards. Parent sampling scans numeric rows and decompresses one card. 5 s busy timeout so the studio, CLI and MCP server can share one database file.
- **Selection:** normalization bounds come from `SUCCESS` cards, so failed cards (score 0) no longer squeeze all working candidates into the top of the range. Failed cards get no score term.
- **Hyperparameters persist:** `rsi_hyperparams_set` writes to a new `settings` table and re-ranks all tasks. Previously the weights lived in process memory, so `rsicraft hyperparams --w-score ...` from the CLI had no lasting effect.
- **Import:** atomic, deferred foreign keys instead of switching enforcement off (an error used to leave `foreign_keys = OFF` on the connection), cards are filed under the bundle's own task, dangling parents are cleared, the frontier is rebuilt.
- **Lineage integrity:** parents must belong to the same task; `CROSSOVER` can no longer receive the same card as both parents; `IMPROVE`/`DEBUG`/`CROSSOVER` return a clear error on a task without the cards they need.
- **Docker sandbox:** now actually passes `--cap-drop ALL`, `--security-opt no-new-privileges`, `--pids-limit 256` and `--user 10000:10001` (the docs claimed the first three, the code did not pass them). Payload goes over stdin instead of an environment variable on the command line (Windows caps that at 32,767 characters). Host-side deadline with `docker kill`. `docker run` infrastructure failures are errors, not experience cards.
- **Local sandbox:** all children run through one captured-output runner with a host-side reaper. Fixes a deadlock where a Rust runner printing more than a pipe buffer was reported as `TIMEOUT`; `rustc` invocations have a deadline; uv-managed Python is found for any 3.x patch version.
- **Metric parsing:** read from evaluator stdout only (a candidate could forge a `RSI_METRIC_JSON` line on stderr that outranked the evaluator's), unknown statuses become `METRIC_PARSE_ERROR` instead of failing the insert, stored logs are capped at 256 KiB.
- **Prompts and extraction:** prompts follow the task language (Rust tasks were prompted for Python); the code-fence parser no longer leaks the info string (` ```rust ` produced a first line `rust`), prefers tagged code blocks, and recovers code from a fence left open by a truncated reply. `max_tokens` raised to 4096 and configurable via `RSICRAFT_MAX_TOKENS`. DEBUG prompts quote the last 6000 bytes of the parent log.
- **Campaign:** DEBUG and CROSSOVER slots fall back to IMPROVE when there is nothing to debug or cross; a failing step returns the completed steps with `aborted: true` instead of discarding them.
- **Diff:** common prefix/suffix trimmed before the LCS table, with a cap on table size.
- **MCP schema:** `sandbox_mode` enum now lists `local`, which the dispatcher already accepted.
- **Docs:** `AGENTS.md` layout corrected (it listed a `rsicraft-store` crate that does not exist and omitted `rsicraft-app`), repo hygiene rules added; `ARCHITECTURE.md` and `CONFIGURATION.md` sandbox claims now match the code.

## 0.2.0 - 2026-11-09

- **Expanded Tool Surface (14 to 22 Tools):** Complete 100% CLI and MCP parity closing all functional gaps:
  - `rsi_diff` (`rsicraft diff --card-a <id> --card-b <id>`): Unified line-by-line diff comparing decompressed CAS source codes between two experience cards.
  - `rsi_llm_probe` (`rsicraft probe [--url <url>]`): Active `/v1/models` reachability, latency in ms, and active model alias inspection.
  - `rsi_hyperparams_get` / `rsi_hyperparams_set` (`rsicraft hyperparams`): Dynamic runtime inspection and adjustment of composite utility weights ($\lambda_s, \lambda_\delta, \lambda_\nu$) without restarts.
  - `rsi_task_delete` (`rsicraft task delete <id>`): Clean cascading task deletion breaking self-referential foreign key cycles.
  - `rsi_cards_prune` (`rsicraft prune --task-id <id> [--keep-frontier <k>]`): Prunes low-utility non-frontier cards while preserving top Pareto performers.
  - `rsi_export` / `rsi_import` (`rsicraft export --task-id <id>`, `rsicraft import --file <path>`): Standalone JSON task bundle export and import with CAS preservation.
  - Multi-language sandbox runner: Added native Rust compilation and evaluation (`rustc --crate-type=rlib candidate.rs` + `rustc evaluator.rs`) alongside Python 3.12.
- **Desktop Studio Enhancements (`rsicraft-app`):**
  - Interactive line-by-line `[Diff with Parent]` viewer embedded directly into the Artifact Inspector.
  - Interactive 3D NVIDIA GeForce RTX 4090 GPU Studio with dual contra-rotating flow-through fans, 12x GDDR6X VRAM chips, AD102 die, and style toggles.
  - 3D Evolutionary Fitness Manifold on the Lineage DAG page with streaming particles and Pareto hypercube wireframe.
  - Updated tool count badges, help reader tables, and smoke check to 22 tools.
- **Philosophical & Engineering Treatises:**
  - `docs/BOSTROM_YUDKOWSKY_AND_RSI_REALITY.md`: Analysis of Bostrom, Yudkowsky, the Foom myth, and physical constraints refuting Skynet.
  - `docs/RSI_ENGINEERING_LIMITS_AND_BENCHMARKING.md`: 5 hard ceilings, 4-stage future trajectory, model feeding matrix, and 4-step benchmarking protocol.
- **Pedagogical 8-Panel Cartoon & YouTube Short Storyboard:**
  - `docs/LITTLE_TIMMY_EXPLAINS_RSI.md`: Standalone illustrated guide written for clever 10-year-olds ("Little Timmy Explains RSICraft"), breaking down the Lego sorting crisis, Frontis the 3B MoE AI kitten, Goliath RTX 4090, isolated Docker playpen (no Skynet), compiler debug loops, genetic crossover, stigmergic memory vault, and the 1.35ms speed demon breakthrough.
  - Complete 5-minute YouTube short storyboard and voiceover script with timestamps, visual cues, and sound effects for YouTube / TikTok audiences.
  - Little Timmy's Big Words Cheat-Sheet (Glossary) demystifying RSI, Pareto Frontier, Stigmergy, Sandbox, CAS, and MoE.
  - Embedded high-res 8-panel comic strip in `README.md`, `HELP.md`, `docs/help/index.html`, and `docs/brochure/rsicraft-brochure.html` (recompiled EPUB & PDF).
- **Fleet Integration:**
  - Added root `start.bat` and `start.ps1` launchers, mirrored to `mcp-central-docs/starts/rsicraft-start.bat`.
  - Rebuilt complete technical brochure suite (`HTML`, `EPUB`, `PDF`) via `BookCraft`.

## 0.1.0 - 2026-11-09

- Initial 3-crate Pure-Rust ArtCraft workspace (`rsicraft-core`, `rsicraft-mcp`, `rsicraft-app`) implementing the RSICraft Technical Implementation Specification for Goliath (RTX 4090 24 GB VRAM + Tsinghua/Frontis.AI `Frontis-MA1` 30B/35B MoE, `arXiv:2607.28568`, `FrontisAI/OpenRSI`).
- `rsicraft-core` (`L1 Core + Store`): Stigmergic SQLite WAL database (`tasks`, `experience_cards`, `task_frontiers`), BLAKE3 + Zstd Content-Addressable Storage, composite utility parent selector (`U_i = 0.50*s_norm + 0.35*delta_norm + 0.15*nu_i`), 4-operator evolutionary loop (`DRAFT`, `IMPROVE`, `DEBUG`, `CROSSOVER`), `llama-server` client with reasoning-tag stripping, isolated Docker / local Python 3.12 / deterministic `demo` sandbox runner, and 9-tab operator manual (`rsi_help`).
- `rsicraft-mcp` (`L2 CLI + MCP`): Single binary exposing 14 MCP `2024-11-05` stdio tools and 1:1 matching CLI subcommands (`--json` + formatted table/help renderer).
- `rsicraft-app` (`L3 Native GPU Desktop Studio`): `eframe`/`egui 0.37` desktop workstation with high-contrast `slate-950` dark & `slate-50` light themes, vector Sun/Moon toggle, `Ctrl+MouseWheel` zoom, compile-time Tofu Glyph Guard (`glyph_allowlist_no_tofu_chars`), Cubic Bezier Lineage DAG Canvas, Score Trajectory Spark Chart, RTX 4090 VRAM gauge, and built-in 9-tab Tsinghua/OpenRSI & Geopolitics dossier.
- Complete documentation stack (`README.md`, `INSTALL.md`, `HELP.md`, `docs/RSI_LANDSCAPE_AND_SKYNET_MYTH.md`, `docs/TSINGHUA_OPENRSI_TEAM.md`, `docs/PRD.md`, `docs/ARCHITECTURE.md`, `docs/TOOLS.md`, `docs/CONFIGURATION.md`, `docs/DEVELOPMENT.md`, `docs/TROUBLESHOOTING.md`) and compiled `BookCraft` brochure suite (`docs/brochure/rsicraft-brochure.{html,epub,pdf}`).
