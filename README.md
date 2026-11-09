# RSICraft

[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
[![ArtCraft 3-Crate](https://img.shields.io/badge/ArtCraft-3--Crate_Standard-0ea5e9.svg)](docs/ARCHITECTURE.md)
[![egui 0.37 GPU Studio](https://img.shields.io/badge/Desktop_UI-eframe%2Fegui_0.37-8b5cf6.svg)](crates/rsicraft-app/src/main.rs)
[![MCP 2024-11-05](https://img.shields.io/badge/MCP-2024--11--05_stdio-10b981.svg)](docs/TOOLS.md)
[![Zero Panics](https://img.shields.io/badge/clippy-zero_unwrap%2Fpanic-22c55e.svg)](Cargo.toml)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

**RSICraft** is a pure-Rust **Sovereign Recursive Self-Improvement (RSI) Studio, CLI & MCP Engine** engineered for consumer 24 GB GPUs (**Goliath**: AMD Ryzen 9 7900X, 64 GB DDR5, NVIDIA GeForce RTX 4090 24 GB VRAM). Built on the scientific and mathematical foundations of **Tsinghua University & Frontis.AI's [`arXiv:2607.28568`](https://arxiv.org/abs/2607.28568) (`Frontis-MA1` / [`FrontisAI/OpenRSI`](https://github.com/FrontisAI/OpenRSI))**, it combines a temperature-calibrated **4-Operator Evolutionary Action Space** (`DRAFT`, `IMPROVE`, `DEBUG`, `CROSSOVER`), an isolated sandbox evaluator (`docker`, `local` Python 3.12, or `demo`), a **Stigmergic SQLite `WAL` + BLAKE3 + Zstd Content-Addressable Store (CAS)**, and a **native GPU-rendered `eframe`/`egui 0.37` Desktop Studio** into a single zero-panic workspace.

---

## Technical Brochure, EPUB Handbook, RSI Geopolitics & Tsinghua Dossier

- 📖 **[Download / View PDF Brochure (`docs/brochure/rsicraft-brochure.pdf`)](docs/brochure/rsicraft-brochure.pdf)** — Print-ready A4 technical dossier with vector L3/L2/L1 architecture diagram, 4-operator action space, live RTX 4090 evolutionary trace, and 14-tool reference matrix.
- 📚 **[Download EPUB eBook (`docs/brochure/rsicraft-brochure.epub`)](docs/brochure/rsicraft-brochure.epub)** — EPUB 3.0 operator handbook for Calibre, Apple Books, Kobo, and mobile e-readers.
- 🌐 **[HTML Brochure Source (`docs/brochure/rsicraft-brochure.html`)](docs/brochure/rsicraft-brochure.html)** — Canonical HTML/CSS source compiled via `BookCraft`.
- 🧭 **[Interactive 9-Tab Help Studio (`docs/help/index.html`)](docs/help/index.html)** | **[Markdown Manual (`HELP.md`)](HELP.md)** | **CLI/MCP (`rsicraft help-tabs` / `rsi_help`)**
- 🌍 **[The Western "Skynet" Panic vs. Closed-Door Frontier RSI (`docs/RSI_LANDSCAPE_AND_SKYNET_MYTH.md`)](docs/RSI_LANDSCAPE_AND_SKYNET_MYTH.md)** — Why Washington/Brussels policy circles frame RSI as "Skynet," and how OpenAI (Preparedness Framework) and Anthropic (ASL-4) keep their internal RSI loops 100% closed.
- 🏛️ **[The Total Expectation Flip: Tsinghua University & Frontis.AI (`docs/TSINGHUA_OPENRSI_TEAM.md`)](docs/TSINGHUA_OPENRSI_TEAM.md)** — How **Prof. Bowen Zhou, Dr. Ning Ding, Kaiyan Zhang, Junlin Yang, Che Jiang**, and their 24-researcher team (`arXiv:2607.28568` / `FrontisAI/OpenRSI`) demystified RSI into an open-weights, reproducible science running on a single consumer RTX 4090.

---

## The Geopolitical & Engineering Expectation Flip

For years, Western AI discourse framed **Recursive Self-Improvement (RSI)** with ominous sci-fi undertones (*"Will it become Skynet?"*). Meanwhile, **OpenAI** (Preparedness Framework *"AI Self-Improvement"* track) and **Anthropic** (Responsible Scaling Policy **ASL-4** *"Automated AI R&D"* trigger) spoke publicly about internal self-improving code loops in Senate hearings and system cards—while keeping every model weight, operator training dataset, and evolutionary harness **100% proprietary** behind closed cloud APIs.

On **July 30, 2026**, a 24-researcher team from **Tsinghua University**, **Frontis.AI**, and **Horizon Research** led by **Junlin Yang, Che Jiang, Yu Fu, Tianwei Luo, Ning Ding, Bowen Zhou, and Kaiyan Zhang** flipped that expectation completely ([`arXiv:2607.28568`](https://arxiv.org/abs/2607.28568), [`FrontisAI/OpenRSI`](https://github.com/FrontisAI/OpenRSI)):
1. **Open Science Over Secrecy:** They formulated RSI ("AI4AI") as an executable, falsifiable optimization loop governed by four explicit atomic program-evolution operators (`Draft`, `Improve`, `Debug`, `Crossover`) and a 3-term composite utility function that prevents population collapse.
2. **Single RTX 4090 Over 10,000-H100 Bunkers:** They benchmarked `Frontis-MA1` + `OpenMLE-Evo` on *MLE-Bench Lite* under a strict consumer hardware cap: **a single NVIDIA RTX 4090 over a 12-hour budget**.
3. **Pure-Rust ArtCraft Hardening (`RSICraft`):** While `FrontisAI/OpenRSI` is a Python research stack, **RSICraft** compiles their exact mathematical and evolutionary architecture into a **zero-panic 3-crate Rust binary + native GPU `egui 0.37` desktop studio + 14-tool MCP server**.

---

## 3-Crate Pure-Rust ArtCraft Architecture (`L1` / `L2` / `L3`)

```text
rsicraft/
├── crates/
│   ├── rsicraft-core/    # L1 Core & Store: SQLite WAL, BLAKE3+Zstd CAS, U_i Utility, 4-Op Engine, LlamaClient, SandboxRunner, 9-Tab Help
│   ├── rsicraft-mcp/     # L2 Binary: Unified CLI (--json / table) + MCP 2024-11-05 stdio server (14 tools)
│   └── rsicraft-app/     # L3 Desktop Studio: Native eframe/egui 0.37 GPU app (Bezier Lineage DAG, Spark Chart, RTX 4090 Gauge, 9-Tab Dossier)
├── sandboxes/
│   └── Dockerfile.evaluator  # Air-gapped rootless Python 3.12 + NumPy/SciPy evaluator image
└── docs/
    ├── brochure/         # rsicraft-brochure.{html,epub,pdf}
    ├── help/             # 9-tab interactive HTML Help Studio (index.html)
    ├── RSI_LANDSCAPE_AND_SKYNET_MYTH.md
    └── TSINGHUA_OPENRSI_TEAM.md
```

### Native `eframe` / `egui 0.37` Desktop Studio (`rsicraft-app`)

Launch with `cargo run -p rsicraft-app`:
- **High-Contrast `slate-950` Dark & `slate-50` Light Themes** with a vector-painted Sun / Crescent Moon toggle, `Ctrl + MouseWheel` GPU zoom, and a strict compile-time Tofu Glyph Guard (`glyph_allowlist_no_tofu_chars`).
- **7 Dedicated Workstation Views:**
  1. **Evolutionary Studio:** One-click `DRAFT`, `IMPROVE`, `DEBUG`, `CROSSOVER`, and `Run 4-Step Campaign` launchpad, live **Evolutionary Score Trajectory Spark Chart** with hover crosshairs, and Materialized Top-K Pareto Frontier cards.
  2. **Lineage DAG Canvas:** Interactive GPU-painted **Cubic Bézier Node Graph** with animated pulse particles connecting `parent_a` and `parent_b` nodes to child experience cards.
  3. **Experience Cards & CAS:** Switchable **Cards (Default) | Table | JSON** browser with side-by-side **BLAKE3 + Zstd Decompressed Source Code & Stack Trace Inspector**.
  4. **Task & Sandbox Lab:** Interactive task editor and live Python candidate sandbox benchmarker (`demo`, `local`, `docker`).
  5. **RTX 4090 & LLM Config:** Visual 24 GB VRAM allocation gauge and endpoint configuration (`llama-server :8081`, Goliath runner, Ollama `:11434`).
  6. **RSI & Tsinghua Dossier:** Built-in deep-dive reader on the Western "Skynet" narrative, closed OpenAI/Anthropic RSI experiments, and the Tsinghua/Frontis.AI `arXiv:2607.28568` breakthrough.
  7. **9-Tab Help & 14 Tools:** Searchable operator manual with one-click PDF/EPUB brochure launchers.

---

## The 4-Operator Action Space & Composite Utility Math

| Operator | Temp | Parent Selection | Evolutionary Role |
| :--- | :---: | :--- | :--- |
| **`DRAFT`** | `0.70` | None (root seed) | High-entropy architectural exploration from the task specification. |
| **`IMPROVE`** | `0.40` | 1 functional parent (`SUCCESS`) | Algorithmic complexity and inner-loop optimization of a working parent. |
| **`DEBUG`** | `0.20` | 1 broken parent (`!= SUCCESS`) | Deterministic stack-trace repair of syntax, runtime, or timeout failures. |
| **`CROSSOVER`** | `0.50` | 2 distinct parents (`parent_a != parent_b`) | Hybrid recombination of the strongest mechanisms from two working parents. |

$$U_i = 0.50 \cdot \text{clamp}\!\left(\frac{s_i - s_{\min}}{s_{\max} - s_{\min}}, 0, 1\right) + 0.35 \cdot \text{clamp}\!\left(\frac{\Delta_i - \Delta_{\min}}{\Delta_{\max} - \Delta_{\min}}, 0, 1\right) + 0.15 \cdot \frac{1}{1 + \text{children\_count}_i}$$

---

## Verified Live on Goliath RTX 4090 (25,000-Element Subarray Benchmark)

Running against live local inference (`llama-server.exe` on Goliath's RTX 4090) and the isolated Python 3.12 sandbox (`sandbox_mode = "local"`), `rsicraft` evolved a 25,000-element maximum contiguous subarray solver (`kadane-4090`):

1. **Step 1 (`DRAFT`, `T=0.70`):** Synthesized a clean $O(N)$ Kadane baseline scoring **`96.5207`** (`1.391 ms` over 80 iterations).
2. **Step 4 (`CROSSOVER`, `T=0.50`):** Sampled two distinct parents (`be4fd318` and `56995273`), identified that Python's built-in `max(x, current_max + x)` function-call overhead inside the 25,000-element loop dominated runtime, and replaced both `max()` calls with inline conditional branching (`if current_sum > 0: current_sum += x else: current_sum = x`).
3. **Result:** Execution time dropped to **`1.359 ms`**, advancing the Pareto frontier to **`96.6028`** (`Rank 1`, `composite_utility = 1.0000`).

---

## Quick Start

```powershell
# 1. Build and run all 6 ArtCraft quality gates
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

# 2. Launch the native GPU Desktop Studio (eframe / egui 0.37)
cargo run -p rsicraft-app

# 3. Seed deterministic offline benchmark ('opt-synth') and inspect top-K frontier
cargo run -p rsicraft-mcp -- --db demo.db demo-seed
cargo run -p rsicraft-mcp -- --db demo.db frontier --task-id opt-synth

# 4. Query the built-in 9-Tab Operator Manual & Tsinghua/OpenRSI Dossier
cargo run -p rsicraft-mcp -- help-tabs --tab rsi_landscape
cargo run -p rsicraft-mcp -- help-tabs --tab tsinghua_openrsi
```

---

## Complete 14-Tool Surface (CLI & MCP `2024-11-05` Parity)

| MCP Tool | CLI Verb | Domain | Mode | Summary |
| :--- | :--- | :--- | :---: | :--- |
| `rsi_status` | `status` | system | Read-Only | SQLite `WAL` store stats, card counts, unique BLAKE3 hashes, RTX 4090 summary. |
| `rsi_hardware_profile` | `hardware` | system | Read-Only | Goliath RTX 4090 (24 GB VRAM) + Frontis-MA1 MoE VRAM budget & launch flags. |
| `rsi_help` | `help-tabs` | system | Read-Only | Query the 9-tab Operator Manual, Skynet vs Closed Labs & Tsinghua Dossier. |
| `rsi_task_upsert` | `task upsert` | tasks | Mutating | Create or update an evolutionary optimization task and evaluator script. |
| `rsi_task_list` | `task list` | tasks | Read-Only | List all registered optimization tasks. |
| `rsi_task_show` | `task show` | tasks | Read-Only | Show task metadata, score/delta bounds, and current top-K frontier. |
| `rsi_step` | `step` | evolution | Mutating | Execute one evolutionary step (`DRAFT`, `IMPROVE`, `DEBUG`, `CROSSOVER`). |
| `rsi_campaign` | `campaign` | evolution | Mutating | Run a multi-step autonomous evolutionary campaign cycling across operators. |
| `rsi_evaluate` | `evaluate` | evolution | Mutating | Benchmark a Python candidate snippet in the sandbox and optionally record it. |
| `rsi_demo_seed` | `demo-seed` | evolution | Mutating | Seed an empty DB with the `opt-synth` benchmark and 4 evolutionary cards. |
| `rsi_get_frontier` | `frontier` | frontier | Read-Only | Return the materialized top-K Pareto frontier experience cards for a task. |
| `rsi_card_list` | `cards` | lineage | Read-Only | List experience cards ordered by composite utility with optional filters. |
| `rsi_card_show` | `card` | lineage | Read-Only | Decompress and display full candidate source code, BLAKE3 hash, and logs. |
| `rsi_lineage` | `lineage` | lineage | Read-Only | Export the evolutionary lineage DAG (nodes and `parent_a`/`parent_b` edges). |

---

## Documentation Index

| Document | Purpose |
| :--- | :--- |
| **[INSTALL.md](INSTALL.md)** | Standalone installation, MCP client configs, RTX 4090 `llama-server` & Docker setup |
| **[HELP.md](HELP.md)** | 9-Tab Operator Manual, Geopolitics Dossier & Competitive Architecture Matrix |
| **[docs/RSI_LANDSCAPE_AND_SKYNET_MYTH.md](docs/RSI_LANDSCAPE_AND_SKYNET_MYTH.md)** | RSI fundamentals, the Western "Skynet" narrative, and closed OpenAI/Anthropic RSI loops |
| **[docs/TSINGHUA_OPENRSI_TEAM.md](docs/TSINGHUA_OPENRSI_TEAM.md)** | Tsinghua University & Frontis.AI team profiles, `arXiv:2607.28568`, and `FrontisAI/OpenRSI` |
| **[docs/brochure/rsicraft-brochure.pdf](docs/brochure/rsicraft-brochure.pdf)** | Printable A4 Technical Brochure & Operator Handbook (PDF) |
| **[docs/brochure/rsicraft-brochure.epub](docs/brochure/rsicraft-brochure.epub)** | EPUB 3.0 Technical Brochure & Operator Handbook |
| **[docs/help/index.html](docs/help/index.html)** | Interactive 9-Tab HTML Help Studio with live search |
| **[docs/PRD.md](docs/PRD.md)** | Product Requirements Document, VRAM mathematics & operator specs |
| **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** | 3-Crate design (`rsicraft-core`, `rsicraft-mcp`, `rsicraft-app`), SQLite `WAL` & safety guards |
| **[docs/TOOLS.md](docs/TOOLS.md)** | Full JSON Schema & CLI examples for all 14 MCP tools |
| **[docs/CONFIGURATION.md](docs/CONFIGURATION.md)** | Environment variables, CLI flags, utility hyperparameters, and sandbox modes |
| **[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)** | Build, test, clippy gates, brochure regeneration, and release workflow |
| **[docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md)** | Incident-grounded runbook (Goliath RTX 4090 2026-11-09 verification fixes) |
| **[docs/SHOW_HN.md](docs/SHOW_HN.md)** | Show HN technical announcement draft |

## License

Dual-licensed under **[MIT](LICENSE-MIT)** or **[Apache-2.0](LICENSE-APACHE)** at your option.
