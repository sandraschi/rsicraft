# RSICraft

Sovereign Recursive Self-Improvement (RSI) Engine in Pure Rust, implementing the Frontis-MA1 (30B/35B MoE, ~3B active) and OpenRSI (OpenMLE-Evo) evolutionary harness for **Goliath** (AMD Ryzen 9 7900X, 64 GB DDR5, NVIDIA GeForce RTX 4090 24 GB VRAM) under the Pure-Rust ArtCraft 3-crate standard.

## Architecture (3-Crate ArtCraft Workspace)

- **`crates/rsicraft-store` (`L1 Store`)**:
  - SQLite database in `WAL` mode (`tasks`, `experience_cards`, `task_frontiers`).
  - Content-Addressable Storage (`cas.rs`) using **BLAKE3** content hashing and **Zstd** (level 3) source code compression.
  - Composite utility parent selection (`utility.rs`):
    `U_i = 0.50 * s_norm_i + 0.35 * delta_norm_i + 0.15 * (1 / (1 + children_count_i))`.
- **`crates/rsicraft-core` (`L1 Core Engine & Command Bus`)**:
  - **4-Operator Evolutionary Action Space (`operators.rs`)**:
    - `DRAFT` (`T = 0.70`): Novel baseline candidate generation.
    - `IMPROVE` (`T = 0.40`): Performance and algorithmic refactoring of functional parents.
    - `DEBUG` (`T = 0.20`): Runtime/compile error patching from sandbox stack traces.
    - `CROSSOVER` (`T = 0.50`): Hybrid recombination of two high-utility parents.
  - **Frontis-MA1 4090 Inference Client (`llm.rs`)**: Queries `llama-server` (`/v1/chat/completions`, Q4_K_M/Q5_K_M, 16k context, `q4_0` KV cache, 18.0-19.5 GB VRAM on RTX 4090) or deterministic `demo` oracle.
  - **Isolated Sandbox Evaluator (`sandbox.rs`)**: Executes candidates in hardened rootless Docker containers (`--network none --memory 4g --cpus 4 --read-only --tmpfs /tmp:rw,noexec,nosuid,size=64m`) or deterministic `demo` mode, parsing `RSI_METRIC_JSON={"status": ..., "score": ...}`.
- **`crates/rsicraft-mcp` (`L2 CLI + MCP Stdio Binary`)**:
  - Single `rsicraft` binary exposing 13 MCP tools over `stdio` (`2024-11-05`) and matching CLI subcommands (`--json`).

## Quick Start

```powershell
# Run full lint, test, and format gates
just ci

# Seed demo benchmark task ('opt-synth') and 4-card evolutionary trajectory
cargo run -p rsicraft-mcp -- --db test_rsi.db demo-seed

# Inspect materialized top-K frontier
cargo run -p rsicraft-mcp -- --db test_rsi.db frontier --task-id opt-synth

# Inspect Goliath RTX 4090 + Frontis-MA1 hardware & VRAM profile
cargo run -p rsicraft-mcp -- hardware --json
```
