# Show HN Draft (Ready When We Flip `sandraschi/rsicraft` Public)

**Title:**
`Show HN: RSICraft – Pure-Rust Recursive Self-Improvement Studio for a single RTX 4090 (arXiv:2607.28568 / OpenRSI)`

---

**Post Body:**

Hi HN,

For the last two years, Recursive Self-Improvement (RSI) in AI has been surrounded by two contradictory narratives in the West:
1. In Washington and Brussels policy hearings, it's framed with ominous sci-fi undertones (*"Will it become Skynet?"*).
2. Inside OpenAI (Preparedness Framework *"AI Self-Improvement"* track) and Anthropic (Responsible Scaling Policy **ASL-4** *"Automated AI R&D"* trigger), internal self-improving code loops are talked about in system cards to justify regulatory moats—while every weight, operator dataset, and evolutionary harness is kept 100% proprietary behind closed cloud APIs.

Then a 24-researcher team from **Tsinghua University** and **Frontis.AI** (Junlin Yang, Che Jiang, Yu Fu, Tianwei Luo, Ning Ding, Bowen Zhou, Kaiyan Zhang et al.) flipped the entire expectation on its head (`arXiv:2607.28568`, `FrontisAI/OpenRSI`). Instead of treating RSI as a classified H100-cluster doomsday project, they formulated "AI4AI" as an executable, falsifiable program-evolution loop governed by four atomic operators (`Draft`, `Improve`, `Debug`, `Crossover`), trained a 35B/30B MoE model (`Frontis-MA1`), open-sourced the stack, and benchmarked it on *MLE-Bench Lite* on a **single consumer 24 GB NVIDIA RTX 4090**.

While their upstream research code is a multi-script Python stack, I wanted a zero-panic, single-binary Rust engine and native GPU desktop studio that runs unattended for days on a local 4090 without losing state or drowning in duplicate `.py` files.

**RSICraft** is a 3-crate Pure-Rust workspace (`#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`):

1. **`rsicraft-core` (Stigmergic SQLite `WAL` Store, BLAKE3+Zstd CAS & 4-Operator Engine):**
   - Source code for every synthesized candidate is hashed with **BLAKE3** (256-bit) and compressed with **Zstd** (level 3) inside a single SQLite `WAL` file (`tasks`, `experience_cards`, `task_frontiers`).
   - Instead of greedy top-1 selection (which collapses evolutionary diversity into a local optimum), parents are sampled via Tsinghua/OpenRSI's 3-term composite utility:
     `U_i = 0.50 * s_norm_i + 0.35 * delta_norm_i + 0.15 * (1 / (1 + children_count_i))`
     Every time a card spawns a child, its `children_count` increments and its novelty bonus decays, naturally pushing search toward under-explored high-performing branches.
   - Implements the 4-operator action space (`DRAFT` at `T=0.70`, `IMPROVE` at `T=0.40`, `DEBUG` at `T=0.20`, `CROSSOVER` at `T=0.50`) against `llama-server` on the RTX 4090 (`Q4_K_M`, 16k context, `q4_0` KV cache, 18.0–19.5 GB VRAM) and evaluates candidates in air-gapped rootless Docker (`--network none --read-only`) or isolated Python 3.12 tempdirs.
2. **`rsicraft-app` (Native `eframe` / `egui 0.37` GPU Desktop Studio):**
   - Renders an interactive **Cubic Bézier Evolutionary Lineage DAG Canvas** with animated pulse particles, an **Evolutionary Score Trajectory Spark Chart**, a live 24 GB RTX 4090 VRAM gauge, a side-by-side decompressed BLAKE3/Zstd code inspector, and a built-in 9-tab dossier.
3. **`rsicraft-mcp` (CLI & MCP `2024-11-05` Stdio Server):**
   - Exposes 14 tools over stdio MCP and 1:1 CLI subcommands (`rsicraft campaign`, `rsicraft step`, `rsicraft frontier`, `rsicraft lineage`, `rsicraft help-tabs`).

### Live Trace on an RTX 4090 (25,000-element Subarray Benchmark)

Running a 4-step live evolutionary campaign on our local RTX 4090 (`llama-server` with speculative decoding + Python 3.12 sandbox on a 25,000-element subarray benchmark over 80 iterations):
- **Step 1 (`DRAFT`, `T=0.70`):** Generated a clean $O(N)$ Kadane implementation scoring **`96.5207`** (`1.391 ms`).
- **Step 4 (`CROSSOVER`, `T=0.50`):** Sampled two parents (`be4fd318` and `56995273`), noticed that Python's built-in `max(x, current_max + x)` function call inside the 25,000-element loop dominated runtime, and replaced both `max()` calls with inline conditional branching (`if current_sum > 0: current_sum += x else: current_sum = x`).
- **Result:** Execution time dropped to **`1.359 ms`**, pushing the primary score to **`96.6028`** (`Rank 1`, `composite_utility = 1.0000`).

Repo: `https://github.com/sandraschi/rsicraft` (MIT/Apache-2.0, includes a `--provider demo --sandbox-mode demo` mode and `--smoke` check so you can test the full evolutionary loop, CLI, MCP server, and `egui` studio in <50ms without a GPU).

Happy to answer any questions about the composite utility math, the Tsinghua `OpenMLE` operator setup, or running 30B/35B MoE evolutionary loops on consumer 24 GB cards!
