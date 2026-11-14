# The Total Expectation Flip: Tsinghua University, Frontis.AI, and `FrontisAI/OpenRSI`

**Navigation:** [README](../README.md) · [Philosophical Foundations](BOSTROM_YUDKOWSKY_AND_RSI_REALITY.md) · [Governance & Safety](RSI_LANDSCAPE_AND_SKYNET_MYTH.md) · [Architecture](ARCHITECTURE.md) · [9-Tab Help Manual](../HELP.md)

---

## 1. The Surprise Drop That Flipped Every Expectation

While Western frontier labs (OpenAI, Anthropic) treated Recursive Self-Improvement (RSI) as a classified, H100-cluster capability too sensitive to release outside proprietary cloud APIs, a collaborative Chinese research team from **Tsinghua University**, **Frontis.AI**, and **Horizon Research** flipped the entire narrative on its head.

On **July 30, 2026**, they published:

- **arXiv Paper:** [*Frontis-MA1: Training an AI4AI Model towards Recursive Self-Improvement in Machine Learning Engineering* (`arXiv:2607.28568`)](https://arxiv.org/abs/2607.28568)
- **Upstream GitHub Repository:** [`https://github.com/FrontisAI/OpenRSI`](https://github.com/FrontisAI/OpenRSI)

Instead of vague press releases or redacted safety scorecards, they released the **complete end-to-end open-source stack (`OpenMLE`)**, the **exact mathematical parent-selection equations**, the **4-operator training alignment**, and **open-weights models (`Frontis-MA1`)**--and proved that high-performing evolutionary self-improvement runs within a **12-hour budget on a single consumer NVIDIA GeForce RTX 4090 GPU**.

---

## 2. Meet the Tsinghua University and Frontis.AI Research Team

The 24-member author team of `arXiv:2607.28568` combines deep expertise in large-model post-training, reinforcement learning, and autonomous agent harnesses from **Tsinghua University** and **Frontis.AI**:

### Full Author Roster (`arXiv:2607.28568`)
> **Junlin Yang, Che Jiang, Yu Fu, Tianwei Luo, Can Ren, Weizhi Wang, Kaikai Zhao, Hongyi Liu, Yuxin Zuo, Yuru Wang, Yuchen Fan, Kai Tian, Zhenzhao Yuan, Xiaojian Lin, Li Sheng, Rushi Qiang, Guoli Jia, Xingtai Lv, Ermo Hua, Dianqiao Lei, Youbang Sun, Ning Ding, Bowen Zhou, and Kaiyan Zhang.**

### Key Principal Investigators and Architects

| Researcher | Affiliation & Background | Core Contributions to OpenRSI & Self-Evolving Agents |
| :--- | :--- | :--- |
| **Prof. Bowen Zhou** | Chair Professor, Dept. of Electronic Engineering, **Tsinghua University**; Dean of **Shanghai AI Lab**; IEEE/CAAI Fellow; former Head of AI Foundations at **IBM Research (Watson)** & Chair of **JD AI Research**. | Senior PI directing the shift from static LLMs to experience-driven self-evolving agents; co-author of both `Frontis-MA1` (`arXiv:2607.28568`) and the landmark taxonomy *"Self-Improving Agents in the Era of Experience: A Survey of Self- to Meta-Evolution"*. |
| **Dr. Ning Ding** | **Tsinghua University (THUNLP)** & **Frontis.AI**; co-founder of **OpenBMB**, lead creator of **UltraFeedback**, **UltraChat**, and the **MiniCPM** edge-model series. | Pioneered high-efficiency open-weights alignment and execution-grounded reinforcement learning so 30B/35B models rival proprietary giants on consumer GPUs. |
| **Kaiyan Zhang** | **Tsinghua University** & **Frontis.AI** senior researcher. | Co-corresponding lead on operator-aligned reinforcement learning (`OpenMLE-ERL`) and verifiable ML engineering gyms (`OpenMLE-Gym`). |
| **Junlin Yang, Che Jiang, Yu Fu & Tianwei Luo** | Lead first authors (**Frontis.AI** & **Tsinghua University**). | Designed and implemented the 4-operator meta-evolution loop (`Draft`, `Improve`, `Debug`, `Crossover`), composite utility parent selection, and the `OpenMLE-Evo` / `OpenMLE-Evo-Max` search frameworks. |

---

## 3. Their Core Objectives and Scientific Philosophy

The Tsinghua and Frontis.AI team articulated three foundational insights in `arXiv:2607.28568`:

### Objective 1: Demystify RSI into Executable, Verifiable "AI4AI"
Rather than speculating about abstract self-awareness, they chose **Machine Learning Engineering (MLE)** and algorithmic optimization as the ideal testbed for RSI because **every candidate program is executable, measurable, and falsifiable**. Either a synthesized script compiles, runs inside a sandbox, and achieves a higher validation/throughput score on held-out data, or it fails with a concrete stack trace.

### Objective 2: Align Model Post-Training with Evolutionary Operators
Previous evolutionary systems (such as DeepMind's *FunSearch* or generic prompt-wrappers around GPT-4/Claude) used general-purpose chat models that were never trained to act as evolutionary operators. When asked to mutate a parent program or crossover two parents, chat models frequently repeat the parent verbatim or introduce trivial syntax errors.

The Tsinghua / Frontis.AI team solved this by aligning **both post-training (`OpenMLE-ERL`) and test-time inference (`OpenMLE-Evo`)** around **four atomic program-evolution operators**:
1. **`DRAFT` (`T = 0.70`)**: Synthesize diverse initial candidate architectures from the problem specification.
2. **`IMPROVE` (`T = 0.40`)**: Refactor and optimize a working parent solution using its execution metrics.
3. **`DEBUG` (`T = 0.20`)**: Repair a broken candidate (`COMPILE_ERROR`, `RUNTIME_ERROR`, `TIMEOUT`) using its exact sandbox stack trace.
4. **`CROSSOVER` (`T = 0.50`)**: Recombine complementary algorithmic mechanisms from two distinct high-utility parent solutions (`parent_a` and `parent_b`).

### Objective 3: Prevent Evolutionary Collapse via Composite Utility and Stigmergic Memory
In long-horizon evolutionary search, naive greedy selection (always mutating the single highest-scoring program) causes rapid **population collapse**--every branch converges onto minor formatting tweaks of the same local optimum.

To maintain long-horizon diversity, `OpenMLE-Evo` introduced:
- **Node Experience Cards** & a **Task-Global Experience Board** (stigmergic memory recording every success, failure, and lineage edge).
- **A 3-Term Composite Utility Function ($U_i$)** balancing **Normalized Score ($0.50$)**, **Normalized Improvement Delta over Parent ($0.35$)**, and an **Exploration Novelty Bonus ($\frac{0.15}{1 + \text{children\_count}_i}$)** that automatically decays as a parent is repeatedly sampled.

---

## 4. How far `RSICraft` has climbed

`FrontisAI/OpenRSI` is a Python research stack whose released runtime is the search loop. RSICraft reimplements that loop in Rust and has partial code for the later rungs. It has not closed the loop.

| Layer | Upstream `FrontisAI/OpenRSI` (Python Reference Stack) | **`RSICraft` today** |
| :--- | :--- | :--- |
| **Rung 1: Evolution** | Python heuristic search | **Real.** Four operators, CAS store, local and Docker sandboxes. The sandbox can repeat a run. Evolutionary steps score once. |
| **Rung 2: Self-Evolution** | Trace logging in the research stack | **Export exists.** SFT and DPO rows are mined from the DAG. Prompts are rebuilt, not the prompts the model saw. |
| **Rung 3: Meta-Evolution** | Fixed operator schedule in the paper's harness | **Advisory statistics only.** Campaigns use a fixed cycle. No Thompson sampling. |
| **Rung 4: Closed-Loop RSI** | Training and eval scripts outside the search loop | **Not closed.** `train_lora.py` has not been run end to end and does not change the active model. The promotion gate compares two served aliases. No adapter is served. |
| **Host Language & Safety** | Multi-script Python orchestrator | **Pure Rust (`#![deny(unwrap, expect, panic, todo)]`)** |
| **State & CAS Store** | Python pickle / state dictionaries | **SQLite `WAL` + 256-bit `BLAKE3` + `Zstd` (L3) CAS (`rsicraft-core::store`)** |
| **RTX 4090 Inference** | vLLM / Python HTTP clients | **Zero-dependency Rust `LlamaClient` (`/v1/chat/completions`, 16k `q4_0` KV)** |
| **Desktop & Agent Surfaces**| CLI scripts only | **Native GPU `egui 0.37` Studio (`rsicraft-app`) + 26-Tool MCP Server (`rsicraft-mcp`)** |
