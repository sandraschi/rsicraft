# Show HN Draft: RSICraft

## Title Options (<= 80 chars, HN format)

1. `Show HN: RSICraft - Sovereign recursive code evolution in pure Rust on one 4090` (79 chars)
2. `Show HN: RSICraft - Pure-Rust OpenRSI harness running 30B MoE on a single 4090` (78 chars)
3. `Show HN: RSICraft - Zero-Python evolutionary code search in Rust (SQLite + CAS)` (79 chars)

## Opening Comment / Post Body

We built `rsicraft` (https://github.com/sandraschi/rsicraft), a single-binary recursive self-improvement (RSI) harness in pure Rust inspired by OpenRSI (OpenMLE-Evo) and Frontis-MA1 (30B/35B MoE, ~3B active parameters per token).

Most evolutionary code-search harnesses today (FunSearch, AlphaEvolve clones, OpenMLE wrappers) require a sprawling Python + Redis + Celery + Ray + Postgres stack on the host just to orchestrate prompts and container runs. Half the time the orchestrator crashes or leaks memory before the search tree reaches depth 20.

We wanted a sovereign, zero-dependency host engine that compiles to one static Rust binary, runs unattended for days on a single consumer workstation (Ryzen 9 7900X, 64 GB RAM, single RTX 4090 24 GB VRAM), and treats every generated candidate as an immutable artifact in a stigmergic database.

### How it works

1. **4-Operator Evolutionary Action Space with Operator-Specific Temperatures:**
   - `DRAFT` (`T = 0.70`): Synthesizes novel baseline candidates from the raw problem spec.
   - `IMPROVE` (`T = 0.40`): Refactors working parents for algorithmic throughput or memory reduction.
   - `DEBUG` (`T = 0.20`): Reads sandbox stack traces/assertion failures and patches broken candidates.
   - `CROSSOVER` (`T = 0.50`): Recombines structural and data-structure choices from two distinct high-utility parents (`parent_a`, `parent_b`).

2. **Stigmergic State Engine (SQLite WAL + BLAKE3 + Zstd CAS):**
   - Every candidate is hashed with BLAKE3 and compressed with Zstd (level 3) into a single SQLite WAL file (`experience_cards` + materialized `task_frontiers`).
   - Instead of greedy top-1 hill-climbing (which collapses into local optima after ~10 iterations), parents are sampled via a composite utility roulette wheel:
     `U_i = 0.50 * s_norm_i + 0.35 * delta_norm_i + 0.15 * (1 / (1 + children_count_i))`
     This rewards both absolute fitness (`s_norm`) and steepness of improvement over its parent (`delta_norm`), while decaying over-sampled nodes (`children_count`) so the search naturally branches.

3. **Single RTX 4090 (24 GB VRAM) Local Inference:**
   - Runs Frontis-MA1 30B/35B MoE (`Q4_K_M` / `Q5_K_M`) via `llama-server` with FlashAttention-2 and `q4_0` KV cache quantization.
   - Total VRAM footprint stays between 18.0 GB and 19.5 GB at a 16k context window, leaving ~4.5 GB headroom on a 24 GB 4090 without host OOMs.

4. **Hardened Container Sandbox + Native MCP (`2024-11-05`) Stdio Server:**
   - Candidates execute inside a rootless, read-only Docker container (`--network none --memory 4g --cpus 4 --read-only --tmpfs /tmp:rw,noexec,nosuid,size=64m`) and emit a deterministic `RSI_METRIC_JSON={"status": ..., "score": ...}` line.
   - Built on our 3-crate ArtCraft containment harness (`rsicraft-store`, `rsicraft-core`, `rsicraft-mcp`) with `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` across the workspace. The same binary runs as a CLI (`--json`) and as an MCP `stdio` server so external agents can inspect the frontier or trigger evolutionary campaigns directly.

5. **First Live 4090 Campaign Trace (`kadane-4090`, 25,000-int benchmark):**
   - `Step 1 (DRAFT, T=0.70)`: Synthesized baseline `O(n)` Kadane loop using Python `max(num, current_sum + num)` (`score = 96.5207`, `1.391 ms`).
   - `Step 2 (IMPROVE, T=0.40)` & `Step 3 (DEBUG, T=0.20)`: Explored alternative loop structures (`93.5537`, `92.5160`), recorded in the stigmergic store with lower delta scores.
   - `Step 4 (CROSSOVER, T=0.50)`: Sampled `parent_a` (`05b6bdd5`) and `parent_b` (`16d8f09e`) via the composite utility wheel and synthesized a hybrid that cached the list reference locally and replaced the inner-loop `max()` function call overhead with inline conditional branching (`if x > new_sum:`), dropping execution time to `1.359 ms` and taking Rank 1 on the frontier (`score = 96.6028`, `composite_utility = 1.0000`).

Happy to answer questions about the composite utility weights, KV-cache sizing on 24 GB cards, or why we moved our entire agent fleet from Python to zero-unwrap Rust workspaces.
