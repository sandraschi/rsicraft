# Show HN: RSICraft -- Pure-Rust Recursive Self-Improvement Engine on a Single RTX 4090

**Title:**
`Show HN: RSICraft – Pure-Rust 4-Rung Recursive Self-Improvement Studio for a Single RTX 4090 (arXiv:2607.28568 / OpenRSI)`

---

**Post Body:**

Hi HN,

For the past two years, discussions around Recursive Self-Improvement (RSI) in AI have been polarized between two extremes:
1. Sci-fi political discourse framing self-improvement as an uncontrollable "Skynet" flash-takeoff (*foom*), motivating proposals for centralized compute licensing.
2. Proprietary industrial research, where labs discuss model autonomy and automated AI R&D in system cards to justify regulatory moats while keeping all weights, operator datasets, and evolutionary harnesses proprietary.

In July 2026, a 24-researcher team from **Tsinghua University** and **Frontis.AI** published an alternative perspective (*Frontis-MA1: Training an AI4AI Model towards Recursive Self-Improvement in Machine Learning Engineering*, `arXiv:2607.28568`, `FrontisAI/OpenRSI`). Rather than treating RSI as an inscrutable supercluster capability, they formulated it as an execution-grounded engineering ladder, trained an open-weights 30B/35B MoE model (`Frontis-MA1`), and demonstrated state-of-the-art results on *MLE-Bench Lite* within a 12-hour budget on a **single consumer 24 GB NVIDIA GeForce RTX 4090**.

**RSICraft** is a pure-Rust implementation and extension of their architecture, built as an ArtCraft three-crate workspace (`#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`):

```text
rsicraft/
├── crates/
│   ├── rsicraft-core/   # L1 Core: SQLite WAL, BLAKE3+Zstd CAS, 4-Rung Frontis Ladder
│   ├── rsicraft-mcp/    # L2 Binary: Unified CLI + MCP 2024-11-05 Stdio Server (26 tools)
│   └── rsicraft-app/    # L3 Studio: Native eframe/egui 0.37 GPU Workstation (Bézier DAG)
```

### Implementing the 4-Rung Frontis Ladder

A common category error is confusing outer-loop program search (such as DeepMind's FunSearch or AIDE) with recursive self-improvement. If model weights and search policies remain identical between step 1 and step 10,000, the improver has not improved. RSICraft implements the full 4-rung ladder to close this loop:

1. **Rung 1: Evolution (Variance-Hardened Program Search):**
   - Four atomic operators: `DRAFT` ($T = 0.70$), `IMPROVE` ($T = 0.40$), `DEBUG` ($T = 0.20$), and `CROSSOVER` ($T = 0.50$).
   - Multi-trial statistical execution ($N \ge 3$ repeated runs) measuring sample mean and sample standard deviation ($s = \sqrt{\frac{1}{N-1}\sum (x_i - \bar{x})^2}$). A candidate is only credited when improvement exceeds the measurement noise floor ($\Delta > 2 \cdot s$).
   - Parent selection via 3-term composite utility:
     $$U_i = 0.50 \cdot \tilde{s}_i + 0.35 \cdot \tilde{\Delta}_i + 0.15 \cdot \frac{1}{1 + \text{children}_i}$$
     Preventing greedy population collapse by decaying novelty as parents are repeatedly sampled.
2. **Rung 2: Self-Evolution (Trace Curation and Dataset Export):**
   - Mined from the SQLite lineage DAG: successful algorithmic improvements and stack-trace error repairs are automatically exported as Supervised Fine-Tuning (SFT) traces.
   - Sibling branches sharing a common parent are extracted as contrastive Direct Preference Optimization (DPO) chosen/rejected pairs.
3. **Rung 3: Meta-Evolution (Dynamic Bayesian Thompson Sampling):**
   - Rather than static round-robin scheduling, operator selection is driven by a Multi-Armed Bandit with Beta conjugate distributions $\text{Beta}(\alpha_k, \beta_k)$.
   - Contextual phase transitions automatically allocate search effort: high-entropy `DRAFT` during cold starts, deterministic `DEBUG` when sandbox error rates spike above 60%, and adaptive exploitation of `IMPROVE`/`CROSSOVER` as candidates approach the Pareto frontier.
4. **Rung 4: Closed-Loop RSI (In-the-Loop LoRA and Promotion Gate):**
   - Curated traces from Rung 2 feed an in-the-loop parameter-efficient fine-tuning (LoRA $r=16, \alpha=32$) script on the RTX 4090, incrementing model generations $G_0 \to G_1 \to \dots \to G_t$.
   - A **Statistical Promotion Gate** benchmarks candidate generation $G_{t+1}$ against baseline $G_t$ across $K=5$ repeated trials on held-out tasks. The model is promoted if $\bar{s}(G_{t+1}) > \bar{s}(G_t) + \epsilon$, with automated rollback if regression or forgetting is observed.

### Architectural Features

- **Single Zero-Panic Binary:** No unhandled panics, zero unwrap/expect in production code.
- **Stigmergic CAS Store:** SQLite with Write-Ahead Logging (`WAL`), 256-bit BLAKE3 content hashing, and Zstd level-3 compression.
- **Air-Gapped Rootless Sandboxing:** Python and Rust candidates run with `--network none`, `--read-only`, `--cap-drop ALL`, `--user 10000:10001`, and a 50 ms polling watchdog reaper.
- **100% CLI / MCP Parity:** All 26 engine tools are exposed identically over MCP 2024-11-05 stdio JSON-RPC and CLI subcommands (`rsicraft campaign`, `rsicraft step`, `rsicraft bandit`, `rsicraft dataset-export`, `rsicraft train`, `rsicraft promote`).
- **GPU Desktop Studio:** Built with `eframe`/`egui 0.37`, featuring an interactive Bézier Lineage DAG canvas, evolutionary trajectory spark charts, a 24 GB VRAM gauge, and a 9-tab operator dossier.

### Verification and Offline Mode

You do not need an RTX 4090 to test the system. RSICraft includes a built-in `--provider demo --sandbox-mode demo` pipeline that simulates the complete 4-rung lifecycle in under 100 milliseconds:
```bash
cargo run -p rsicraft-mcp -- demo-seed --json
cargo run -p rsicraft-mcp -- bandit --task-id opt-synth --json
cargo run -p rsicraft-mcp -- dataset-export --task-id opt-synth --format all --json
cargo run -p rsicraft-mcp -- train --task-id opt-synth --dry-run --json
cargo run -p rsicraft-mcp -- promote --task-id opt-synth --trials 5 --json
```

Repository: `https://github.com/sandraschi/rsicraft` (MIT/Apache-2.0)

We would love feedback from folks working on automated program synthesis, Bayesian search policies, or consumer LLM fine-tuning loops.
