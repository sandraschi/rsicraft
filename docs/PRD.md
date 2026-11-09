# Product Requirements Document (PRD): RSICraft (`v0.2.0`)

**Navigation:** [README](../README.md) · [RSI & The "Skynet" Myth](RSI_LANDSCAPE_AND_SKYNET_MYTH.md) · [Tsinghua & OpenRSI Dossier](TSINGHUA_OPENRSI_TEAM.md) · [Architecture](ARCHITECTURE.md)

---

## 1. Mission Statement

Build **`rsicraft`**, a pure-Rust, zero-panic sovereign Recursive Self-Improvement (RSI) studio, CLI, and MCP server derived from the architectural principles of **Frontis-MA1 (30B/35B MoE)** and **OpenRSI (`OpenMLE-Evo`)** by researchers at **Tsinghua University**, **Frontis.AI**, and **Horizon Research** ([`arXiv:2607.28568`](https://arxiv.org/abs/2607.28568), [`FrontisAI/OpenRSI`](https://github.com/FrontisAI/OpenRSI)).

The system manages an open-ended evolutionary search loop over executable code candidate spaces on a single consumer 24 GB GPU (**Goliath**: AMD Ryzen 9 7900X, 64 GB DDR5, NVIDIA GeForce RTX 4090 24 GB VRAM), maintaining persistent stigmergic state in SQLite `WAL` with BLAKE3+Zstd Content-Addressable Storage (CAS), running isolated execution evaluations in sandboxed containers or local Python 3.12 environments, and exposing control via:
1. **`rsicraft-app` (`L3`)**: A native GPU-rendered `eframe`/`egui 0.37` Desktop Studio with an interactive Bézier Lineage DAG Canvas, 4-operator launchpad, score spark charts, and 9-tab dossier.
2. **`rsicraft-mcp` (`L2`)**: A unified CLI (`--json` + formatted tables) and MCP `2024-11-05` stdio server exposing 22 tools.
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
   - Active LLM endpoint reachability and latency probing (`rsi_llm_probe`).
   - Runtime composite utility hyperparameter inspection and tuning (`rsi_hyperparams_get`, `rsi_hyperparams_set`).
   - Complete task lineage JSON export and import bundles (`rsi_export`, `rsi_import`).
   - Cascading task deletion (`rsi_task_delete`) and Pareto frontier card pruning (`rsi_cards_prune`).
   - Native Rust compilation and evaluation (`rustc --crate-type=rlib`) alongside Python 3.12.
5. **6 Non-Negotiable ArtCraft Merge Gates**:
   - Gate 1: `cargo clippy --workspace --all-targets --all-features -- -D warnings` (0 warnings, 0 `unwrap`/`expect`/`panic`/`todo`).
   - Gate 2: `cargo test --workspace` (100% green including `glyph_allowlist_no_tofu_chars`).
   - Gate 3: `cargo fmt --all --check`.
   - Gate 4: MCP `2024-11-05` stdio probe (`initialize`, `tools/list` = 22 tools, `tools/call`).
   - Gate 5: Committed `Cargo.lock`.
   - Gate 6: Complete README stack, 9-tab help, and `BookCraft` brochure (`HTML`, `EPUB`, `PDF`).
