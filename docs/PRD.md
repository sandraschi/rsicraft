# Product Requirements Document (PRD): RSICraft (`v0.3.1`)

**Navigation:** [README](../README.md) · [RSI & The "Skynet" Myth](RSI_LANDSCAPE_AND_SKYNET_MYTH.md) · [Tsinghua & OpenRSI Dossier](TSINGHUA_OPENRSI_TEAM.md) · [Architecture](ARCHITECTURE.md)

---

## 1. Mission Statement

Build **`rsicraft`**, a pure-Rust, zero-panic studio, CLI, and MCP server for **LLM-guided evolutionary program search**, and use it as a bench to test, with measurements, how far the self-improvement ladder of **Frontis-MA1 / OpenRSI (`OpenMLE-Evo`)** by **Tsinghua University**, **Frontis.AI**, and **Horizon Research** ([`arXiv:2607.28568`](https://arxiv.org/abs/2607.28568), [`FrontisAI/OpenRSI`](https://github.com/FrontisAI/OpenRSI)) can be climbed on one consumer GPU.

**What the product is today:** all three crates. `rsicraft-core` holds the store, the four operators, the sandboxes, trace export, operator statistics, the LoRA harness, the promotion gate, and classified LLM errors (a campaign preflights the provider, and a step with no top-level function is `no_code`). `rsicraft-mcp` is the 26-tool CLI and MCP server. `rsicraft-app` is the egui studio (lineage DAG, live run, seven pages, the same 10-tab manual as `rsicraft help-tabs`). **What has not happened:** no generation has been trained, and campaigns still use a fixed operator cycle. The studio has the 8-menu bar, store undo and redo, and file dialogs for Open Database and Save As. Cut, paste, and select all have no selection. Recursive self-improvement is the bench question. The paper itself claims meta-evolution with one generation and disclaims general RSI. Status and measurements: [FRONTIS_PARITY_PLAN.md](FRONTIS_PARITY_PLAN.md), [BENCH_MODELSWAP_V2.md](BENCH_MODELSWAP_V2.md), [BENCH_AMP_V2.md](BENCH_AMP_V2.md).
The system manages an open-ended evolutionary search loop over executable code candidate spaces on a single consumer 24 GB GPU (**Goliath**: AMD Ryzen 9 7900X, 64 GB DDR5, NVIDIA GeForce RTX 4090 24 GB VRAM), maintaining persistent stigmergic state in SQLite `WAL` with BLAKE3+Zstd Content-Addressable Storage (CAS), running isolated execution evaluations in sandboxed containers or local Python 3.12 environments, and exposing control via:
1. **`rsicraft-app` (`L3`)**: A native GPU-rendered `eframe`/`egui 0.37` Desktop Studio with an interactive Bézier Lineage DAG Canvas, 4-operator launchpad, score spark charts, and the 10-tab manual (Help page) plus a "What is RSI?" page.
2. **`rsicraft-mcp` (`L2`)**: A unified CLI (`--json` + formatted tables) and MCP `2024-11-05` stdio server exposing 26 tools.
3. **`rsicraft-core` (`L1`)**: The deterministic evolutionary state machine, composite utility selector, and store.

---

## 2. Target Hardware & VRAM Budget (Goliath RTX 4090)

| Resource | Specification | Allocation / Bound |
| :--- | :--- | :--- |
| **CPU & Host RAM** | AMD Ryzen 9 7900X (24 threads), 64 GB DDR5 | 4 CPU cores & 4 GB RAM allocated per sandbox evaluation |
| **GPU Hardware** | NVIDIA GeForce RTX 4090 (`24.0 GB` GDDR6X) | `18.0 GB – 19.5 GB` total VRAM ceiling (`> 4.5 GB` safety headroom) |
| **Model Architecture** | `Frontis-MA1` 30B/35B MoE (~3B active/token) | `Q4_K_M` / `Q5_K_M` GGUF (`--n-gpu-layers 99`) |
| **Context & KV Cache** | `16,384` tokens (`--parallel 2`) | `--cache-type-k q4_0 --cache-type-v q4_0 --flash-attn` |

---

## 3. Core Functional Requirements

1. **4-Operator Evolutionary Action Space (`operators.rs`)**:
   - `DRAFT` (`T = 0.70`): High-entropy baseline synthesis from raw problem specifications.
   - `IMPROVE` (`T = 0.40`): Algorithmic and constant-factor optimization of a sampled `SUCCESS` parent.
   - `DEBUG` (`T = 0.20`): Stack-trace-driven repair of a sampled broken parent (`COMPILE_ERROR`, `RUNTIME_ERROR`, `TIMEOUT`).
   - `CROSSOVER` (`T = 0.50`): Recombination of two distinct high-utility parents (`parent_a != parent_b`).
2. **Composite Utility Parent Selection (`store/utility.rs`)**:
   - $U_i = 0.50 \cdot s_{\text{norm}, i} + 0.35 \cdot \Delta_{\text{norm}, i} + 0.15 \cdot \frac{1}{1 + \text{children\_count}_i}$.
   - Stochastic roulette-wheel sampling proportional to $U_i$.
3. **Stigmergic SQLite `WAL` + BLAKE3/Zstd CAS (`store/mod.rs`, `store/cas.rs`)**:
   - 256-bit BLAKE3 content hashing + Zstd level-3 compression of candidate source code.
   - Materialized `task_frontiers` table tracking the top-K Pareto frontier per task.
4. **Extended Tooling & Multi-Language Sandboxing**:
   - Line-by-line unified code diffing (`rsi_diff`) between CAS experience cards.
   - LLM diagnosis (`rsi_llm_probe`): provider, whether the configured model is served, and warnings. The call itself does not fail.
   - Runtime composite utility hyperparameter inspection and tuning (`rsi_hyperparams_get`, `rsi_hyperparams_set`).
   - Complete task lineage JSON export and import bundles (`rsi_export`, `rsi_import`).
   - Cascading task deletion (`rsi_task_delete`) and Pareto frontier card pruning (`rsi_cards_prune`).
   - Native Rust compilation and evaluation (`rustc --crate-type=rlib`) alongside Python 3.12.
5. **6 Non-Negotiable ArtCraft Merge Gates**:
   - Gate 1: `cargo clippy --workspace --all-targets --all-features -- -D warnings` (0 warnings, 0 `unwrap`/`expect`/`panic`/`todo`).
   - Gate 2: `cargo test --workspace` (100% green including `glyph_allowlist_no_tofu_chars`).
   - Gate 3: `cargo fmt --all --check`.
   - Gate 4: MCP `2024-11-05` stdio probe (`initialize`, `tools/list` = 26 tools, `tools/call`).
   - Gate 5: Committed `Cargo.lock`.
   - Gate 6: Complete README stack, 10-tab help, and `BookCraft` brochure (`HTML`, `EPUB`, `PDF`).

---

## 4. UI/UX Architecture & Menu Bar Standard

**Current studio:** a top menu bar (`File`, `Edit`, `View`, `Insert`, `Modify`, `Commands`, `Window`, `Help`) plus seven sidebar pages: Evolutionary Studio, Lineage DAG Canvas, Experience Cards, Task and Sandbox Lab, RTX 4090 and LLM, What is RSI?, Help. The Help page is the 10-tab manual. "What is RSI?" shows the `rsi` and `based_on` tabs. Help dialogs: Command Palette, Keyboard Shortcuts, Project Files and Docs, Licenses and Notices, About RSICraft. Engine entries call `Command::dispatch`. Undo and redo restore the last store snapshot (task edits, prune, seed, a committed step, Open, Save As), up to 8 deep, and wait while a run is in progress. Open Database and Save As open a file dialog. Save checkpoints a file-backed store, or asks for a path when the store is in memory. Cut, paste, and select all still have no selection. Copy places the selected card source on the clipboard. The list below is the menu that is in the binary.

In accordance with [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md):
- **Pattern 2 (Viewport-Centric Craft Workstation):** Persistent Bézier Lineage DAG canvas and candidate code evaluation stage. Top quick button row acts as accelerators into the `View` menu. To add a new search operator or telemetry dock:
  1. Register a visibility toggle in the `View` or `Window` menu.
  2. Add an entry in the sidebar Operator / Candidate toolbox.
  3. Wire the action directly into the universal `Command::dispatch` bus.
- **Top Menu Bar Hierarchy:**
  - `File`: New Task (`Ctrl+N`), Open Database (`Ctrl+O`), Open Recent (none), Save (`Ctrl+S`), Save As (`Ctrl+Shift+S`), Export Lineage (`Ctrl+E`), Close Task (`Ctrl+W`), Quit (`Ctrl+Q`).
  - `Edit`: Undo (`Ctrl+Z`), Redo (`Ctrl+Y`), Cut, Copy card source (`Ctrl+C`), Paste, Select All, Preferences (`Ctrl+,`).
  - `View`: Reset Zoom, the seven pages, Theme Dark, Theme Light.
  - `Insert`: Add Seed Candidate, Insert Problem Specification.
  - `Modify`: Mutate Candidate (IMPROVE), Repair Candidate (DEBUG), Cross Parents (CROSSOVER), Prune Frontier, Show Utility Weights.
  - `Commands`: Run Evolutionary Loop, Probe LLM Endpoint, Evaluate Candidate Sandbox, Calculate Pareto Frontier.
  - `Window`: Reset Panel Layout, Lineage DAG Viewport, Candidate Diff Inspector, Live Run.
  - `Help` (Mandatory 5-Dialog Suite): `Command Palette...` (`Ctrl+K`), `Keyboard Shortcuts...` (`F1`), `Project Files & Docs...`, `Licenses & Notices...`, `About RSICraft`.
- **Behavioral Mechanics:** Smooth `switch_on_hover` between top-level menus, zero-unsafe immediate-mode rendering, and headless `--smoke` parity.

---

## 5. Research Goals, Gates and Non-Goals (added 2026-11-12)

**Goals, in order. Each one is gated by the one before it.**

| # | Goal | Gate to pass | State |
| :-- | :--- | :--- | :--- |
| G1 | A reliable search tool: campaigns run unattended, every step yields a card or a recorded reason | Under 5% cardless steps on the reference models | Not met: 25 to 33% cardless steps on the 35B models. pack-v2 on the 9B model aborted or returned no success card on 13 of 15 runs |
| G2 | A task pack where search clearly beats one-shot answers | **Amplification gate:** median (final best / first working candidate) of 1.5 or more over at least 3 trials, above trial-to-trial spread | Failed. pack-v1 median 1.18. pack-v2 measured 2026-11-12: 2 of 15 runs produced a success card. See [BENCH_AMP_V2.md](BENCH_AMP_V2.md) |
| G3 | Fair model comparison on a qualifying pack | Identical serving parameters for both models, at least 3 trials per cell | Not started on pack-v2 |
| G4 | One trained generation: G1 better than G0 on held-out tasks | `rsi_promote_gate` on held-out tasks only, Welch t of 2 or more; metrics are share of valid candidates, steps to threshold, best score at N steps | Not started. Needs adapter serving and a held-out split in the store |
| G5 | A second generation trained on the first one's traces | Same gate, G2 against G1 | Research, no commitment |

**Reporting rules (requirements, not style):**
- A result is reported with its trial count and spread, or it is not reported.
- Training on a larger model's traces is called distillation, never self-improvement.
- A gate that fails is written down as a result. The docs are corrected in the same change.
- No document may claim a rung of the ladder that section 2 of the plan marks as missing.

**Non-goals:**
- General or autonomous RSI. Nobody has it, including the paper.
- Copying OpenRSI code. It is CC BY-NC 4.0; rsicraft is MIT/Apache. Ideas are reimplemented from the paper.
- Fine-tuning a 35B model on the 24 GB card. The loop experiment runs on a 9B-class model.
- GPU workloads inside the sandbox while a 35B model holds the card.

**Functional requirements that follow from the goals:**
1. Task packs as plain files (`tasks/<pack>/<task>.py`, metadata in header comments, `split: train|heldout`), loadable without code changes.
2. `scripts/bench-models.ps1` (resumable, one database and one campaign report per run) and `scripts/bench-analyze.ps1` (first / best / amplification).
3. Per-card token and time accounting; a `runs` table.
4. Campaign variants for ablation (operator set, parent-selection policy, scoring trials), selectable per run.
5. Held-out split known to the store and enforced by the promotion gate.
6. Adapter serving path (LoRA to GGUF, registered alias) so a trained generation can be tested.
