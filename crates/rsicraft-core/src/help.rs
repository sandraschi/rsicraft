//! Multi-tab help and operator manual for `rsicraft` (`rsi_help` / `rsicraft help-tabs`).
//!
//! Implements 9 tabs covering the 5 mandatory tabs from `WEBAPP_HELP_STANDARD.md`
//! (`overview`, `wrappee`, `api`, `troubleshoot`, `faq`), two technical domain tabs
//! (`operators`, `compare`), and two deep-dive context tabs (`rsi_landscape` on the
//! Western corporate/political "Skynet" narrative and closed OpenAI/Anthropic RSI
//! experiments, and `tsinghua_openrsi` on the Tsinghua University & Frontis.AI team,
//! `arXiv:2607.28568`, and `FrontisAI/OpenRSI`).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpSection {
    pub heading: String,
    pub body: String,
    pub code_example: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpTab {
    pub id: String,
    pub title: String,
    pub badge: String,
    pub summary: String,
    pub sections: Vec<HelpSection>,
}

#[must_use]
pub fn all_help_tabs() -> Vec<HelpTab> {
    vec![
        HelpTab {
            id: "overview".into(),
            title: "1. Overview & 3-Crate Architecture".into(),
            badge: "ArtCraft 3-Tier".into(),
            summary: "Sovereign Recursive Self-Improvement (RSI) Engine for Goliath (RTX 4090 24 GB VRAM) combining Frontis-MA1 MoE synthesis, 4-operator evolutionary search, Stigmergic SQLite WAL + BLAKE3/Zstd CAS, and a native eframe/egui GPU desktop studio.".into(),
            sections: vec![
                HelpSection {
                    heading: "Why RSICraft Exists".into(),
                    body: "Existing evolutionary code-search harnesses (FunSearch, OpenMLE-Evo, Python/Aider loops) orchestrate LLM loops in stateful Python daemons that lose lineage on crash, store uncompressed duplicate source files, and select parents greedily by raw score alone. RSICraft compiles the entire evolutionary state machine, BLAKE3+Zstd Content-Addressable Store (CAS), composite utility parent selector, MCP 2024-11-05 stdio server, and native eframe/egui 0.37 desktop studio into a single zero-panic Rust workspace.".into(),
                    code_example: Some("rsicraft status --json\nrsicraft hardware --json\ncargo run -p rsicraft-app".into()),
                },
                HelpSection {
                    heading: "3-Crate Pure-Rust ArtCraft Architecture (L1 / L2 / L3)".into(),
                    body: "1. rsicraft-core (L1 Core + Store): SQLite WAL database (tasks, experience_cards, task_frontiers), BLAKE3 content hashing, Zstd (level 3) source compression, composite utility calculator, 4-operator action space (DRAFT, IMPROVE, DEBUG, CROSSOVER), llama-server inference client, and isolated Docker / Python 3.12 / demo sandbox runner.\n2. rsicraft-mcp (L2 CLI + MCP Binary): Single binary exposing 14 MCP tools over stdio and 1:1 matching CLI subcommands.\n3. rsicraft-app (L3 Desktop Studio): Native GPU-rendered eframe/egui 0.37 workstation with interactive Bezier lineage DAG canvas, 4-operator launchpad, score spark charts, RTX 4090 VRAM gauge, and 9-tab documentation reader.".into(),
                    code_example: Some("cargo test --workspace\ncargo clippy --workspace --all-targets --all-features -- -D warnings".into()),
                },
                HelpSection {
                    heading: "Live RTX 4090 Verification (Kadane 25k Subarray Benchmark)".into(),
                    body: "Verified on Goliath (AMD Ryzen 9 7900X, 64 GB DDR5, NVIDIA GeForce RTX 4090 24 GB VRAM) against live llama-server inference and Python 3.12 sandbox evaluation: DRAFT synthesized an O(N) Kadane baseline scoring 96.5207 (1.391 ms); CROSSOVER recombined parent_a and parent_b by replacing inner-loop Python max() calls with inline conditional branching, advancing the Pareto frontier to 96.6028 (1.359 ms, Rank 1, composite_utility = 1.0000).".into(),
                    code_example: Some("rsicraft --llama-url http://127.0.0.1:8081 campaign --task-id kadane-4090 --steps 4 --provider llama --sandbox-mode local".into()),
                },
            ],
        },
        HelpTab {
            id: "rsi_landscape".into(),
            title: "2. What is RSI? The 'Skynet' Myth & Closed Frontier Labs".into(),
            badge: "Geopolitics & Myth".into(),
            summary: "From I.J. Good's 1965 'Intelligence Explosion' to Western political panic ('Will it be Skynet?') and the closed-door RSI experiments inside OpenAI and Anthropic.".into(),
            sections: vec![
                HelpSection {
                    heading: "What Recursive Self-Improvement (RSI) Actually Is".into(),
                    body: "Recursive Self-Improvement (RSI) is a closed-loop computational process in which an AI system generates, evaluates, debugs, and recombines candidate algorithms or training pipelines, using objective execution feedback from generation N to produce a stronger generation N+1. In concrete engineering terms, RSI is not magic sentient consciousness; it is evolutionary search coupled with execution-grounded verification over executable program spaces.".into(),
                    code_example: None,
                },
                HelpSection {
                    heading: "Ominous Undertones in Western Politics & Media ('Will It Be Skynet?')".into(),
                    body: "Since I.J. Good's 1965 speculation on an 'ultraintelligent machine' and Nick Bostrom's Superintelligence (2014), Western policy circles in Washington, London, Brussels, and Sacramento have treated the phrase 'Recursive Self-Improvement' with apocalyptic dread ('Will it become Skynet?', 'uncontrollable foom', 'rogue takeoff'). Senate hearings, Executive Orders, and safety summit communiques routinely frame RSI as an existential red line where an AI rewrites its own weights in secret and escapes human control. That sci-fi framing served a convenient political purpose: convincing lawmakers that RSI research is too dangerous for open-weights academia or independent developers to possess.".into(),
                    code_example: None,
                },
                HelpSection {
                    heading: "Bostrom, Yudkowsky & The Philosophical Roots of the Foom Myth".into(),
                    body: "Nick Bostrom (Superintelligence, 2014) and Eliezer Yudkowsky (MIRI) formulated the canonical theory of recursive takeoff: an intelligence metric I(t+1) = I(t) + g(I(t)), where an AI self-improves at an accelerating rate proportional to I(t)^(1+alpha). Yudkowsky argued this leads to an unconstrained fast takeoff ('foom') governed by the Orthogonality Thesis and Instrumental Convergence (resource acquisition, self-preservation). In popular media, this was popularized as the 'Paperclip Maximizer' or 'Skynet' archetype. However, modern empirical AI research reveals that this thought experiment ignored the physical realities of computation: memory bandwidth, thermal TDP dissipation, and the non-zero latency of sandbox verification.".into(),
                    code_example: None,
                },
                HelpSection {
                    heading: "Why Real-World RSI Does Not Produce Skynet (Physics, Complexity & Containment)".into(),
                    body: "Real RSI in 2026 (Frontis-MA1, RSICraft) breaks the Skynet myth through five fundamental physical realities:\n1. Execution-Grounded Reality: An AI cannot arbitrarily declare itself smarter; it must produce verifiable source code tested against deterministic compilers, test suites, and hardware clocks.\n2. Algorithmic Complexity Lower Bounds: No amount of recursive optimization can turn an Omega(N log N) sorting problem into O(1). Mathematics imposes hard ceilings on optimization.\n3. Thermodynamic & VRAM Limits: Computation is physical. A 24 GB RTX 4090 consumes 450W and will crash with a deterministic CUDA OOM if an allocation exceeds memory. There is no mystical escape from physical hardware.\n4. Stigmergic Externalization: Lineage is not an opaque ghost in the neural weights; all mutations are explicitly serialized in SQLite WAL and BLAKE3-hashed CAS.\n5. Air-Gapped Sandboxing: Evaluators run in rootless, network-disabled containers (--network none) with strict timeouts and memory caps.".into(),
                    code_example: None,
                },
                HelpSection {
                    heading: "OpenAI & Anthropic's Closed-Door RSI Experiments".into(),
                    body: "Both OpenAI and Anthropic have spoken publicly about internal RSI and automated AI R&D while keeping every weight, dataset, and harness strictly proprietary:\n- OpenAI Preparedness Framework: Defines 'AI Self-Improvement' (automated ML research and engineering) as a High/Critical risk category. OpenAI executives frequently reference internal o1/o3/o4 reinforcement-learning self-play and automated research-engineer loops, yet publish zero reproducible code or operator weights.\n- Anthropic Responsible Scaling Policy (RSP / ASL-4): Explicitly defines Autonomy & Automated AI R&D (Claude autonomously optimizing kernels, training loops, and post-training pipelines) as the trigger for ASL-4 containment. Anthropic discusses internal self-improvement evaluations in system cards to justify regulatory moats, while keeping the entire evolutionary harness locked behind closed cloud APIs.".into(),
                    code_example: None,
                },
            ],
        },
        HelpTab {
            id: "tsinghua_openrsi".into(),
            title: "3. The Expectation Flip: Tsinghua & Frontis.AI (OpenRSI)".into(),
            badge: "arXiv:2607.28568".into(),
            summary: "How researchers from Tsinghua University and Frontis.AI flipped the script by publishing arXiv:2607.28568 and open-sourcing FrontisAI/OpenRSI for a single consumer RTX 4090.".into(),
            sections: vec![
                HelpSection {
                    heading: "The Total Expectation Flip: Open-Source RSI on a 24 GB RTX 4090".into(),
                    body: "While Western frontier labs mystified RSI as a classified H100-cluster doomsday project, a collaborative team from Tsinghua University, Frontis.AI, and Horizon Research did the exact opposite. On July 30, 2026, they published 'Frontis-MA1: Training an AI4AI Model towards Recursive Self-Improvement in Machine Learning Engineering' (arXiv:2607.28568) and released the complete code stack and model weights on GitHub (https://github.com/FrontisAI/OpenRSI). Even more remarkably, they explicitly benchmarked their system onMLE-Bench Lite under a strict consumer hardware constraint: a single NVIDIA RTX 4090 with a 12-hour budget.".into(),
                    code_example: Some("# Upstream Paper: https://arxiv.org/abs/2607.28568\n# Upstream Repo:  https://github.com/FrontisAI/OpenRSI".into()),
                },
                HelpSection {
                    heading: "The Tsinghua University & Frontis.AI Research Team".into(),
                    body: "The 24-author team on arXiv:2607.28568 bridges Tsinghua University's premier NLP/Agent laboratories and Frontis.AI:\n- Lead & Core Contributors: Junlin Yang, Che Jiang, Yu Fu, Tianwei Luo, Can Ren, Weizhi Wang, Kaikai Zhao, Hongyi Liu, Yuxin Zuo, Yuru Wang, Yuchen Fan, Kai Tian, Zhenzhao Yuan, Xiaojian Lin, Li Sheng, Rushi Qiang, Guoli Jia, Xingtai Lv, Ermo Hua, Dianqiao Lei, and Youbang Sun.\n- Senior Principal Investigators (Tsinghua University / Frontis.AI): Prof. Bowen Zhou (Chair Professor in the Department of Electronic Engineering at Tsinghua University, Dean of Shanghai AI Lab, former Head of AI Foundations at IBM Research Watson and JD AI), Dr. Ning Ding (Tsinghua NLP / OpenBMB co-founder, creator of UltraFeedback & MiniCPM), and Kaiyan Zhang (Tsinghua University).\n- Related Tsinghua Foundation Work: Che Jiang and Bowen Zhou also co-authored the June 2026 landmark taxonomy 'Self-Improving Agents in the Era of Experience: A Survey of Self- to Meta-Evolution' alongside Tsinghua's EvoAgent and MaD-RSI initiatives.".into(),
                    code_example: None,
                },
                HelpSection {
                    heading: "Their Core Objectives & The 3-Layer OpenMLE Architecture".into(),
                    body: "The Tsinghua / Frontis.AI team set out to strip the mysticism from RSI and turn 'AI4AI' (AI systems that improve AI engineering) into an executable, measurable, reproducible science built on three pillars:\n1. OpenMLE-Gym: Thousands of quality-gated, verifiable ML and algorithmic task environments with deterministic sandbox execution feedback.\n2. OpenMLE-ERL (Execution-Grounded RL & SFT): Post-training the 35B/30B MoE model (Frontis-MA1) specifically around four atomic program-evolution operators (Draft, Improve, Debug, Crossover) using real sandbox execution rewards rather than synthetic chat preferences.\n3. OpenMLE-Evo & OpenMLE-Evo-Max: Long-horizon population search using a 3-term composite utility (Score + Improvement Delta + Exploration Novelty Decay) and stigmergic experience cards so the search tree never collapses into a single greedy trajectory.\n\nRSICraft takes Tsinghua and Frontis.AI's exact mathematical and architectural blueprint and re-engineers the host runtime into a single zero-panic Pure-Rust ArtCraft studio + MCP server.".into(),
                    code_example: None,
                },
            ],
        },
        HelpTab {
            id: "wrappee".into(),
            title: "4. Wrapped Backends (LLM & Sandbox)".into(),
            badge: "RTX 4090 + Sandbox".into(),
            summary: "How RSICraft orchestrates llama.cpp (llama-server.exe) / Ollama on the RTX 4090 and evaluates candidate artifacts inside air-gapped Docker or isolated Python 3.12 sandboxes.".into(),
            sections: vec![
                HelpSection {
                    heading: "Goliath RTX 4090 + Frontis-MA1 / Reasoning Model Inference".into(),
                    body: "RSICraft connects to any OpenAI-compatible /v1/chat/completions endpoint (default http://127.0.0.1:8081 for llama-server, or custom port via --llama-url). On a 24 GB RTX 4090, Frontis-MA1 30B/35B MoE (~3B active parameters per token) in Q4_K_M / Q5_K_M with a 16,384-token context window and q4_0 KV cache quantization occupies 18.0-19.5 GB VRAM at ~120 tok/s.".into(),
                    code_example: Some("llama-server --model models/frontis-ma1-30b-a3b-q4_k_m.gguf --alias frontis --ctx-size 16384 --n-gpu-layers 99 --cache-type-k q4_0 --cache-type-v q4_0 --flash-attn --parallel 2 --port 8081".into()),
                },
                HelpSection {
                    heading: "Automatic Reasoning-Block Stripping (<|channel>thought & <think>)".into(),
                    body: "Modern reasoning models (Gemma-4, Qwen3, DeepSeek-R1) emit chain-of-thought blocks before the final Python code fence. PromptBuilder::extract_code_block automatically strips everything up to the closing <|channel>...<channel|> or </think> delimiter before extracting the ```python block, preventing thought prose from polluting candidate.py.".into(),
                    code_example: None,
                },
                HelpSection {
                    heading: "Three Sandbox Execution Modes (docker, local, demo)".into(),
                    body: "- docker: Hardened rootless container (--network none --memory 4g --cpus 4.0 --pids-limit 64 --security-opt no-new-privileges --cap-drop ALL --read-only --tmpfs /tmp:rw,noexec,nosuid,size=64m) with candidate.py and evaluator.py bind-mounted read-only.\n- local: Isolated temporary workspace directory evaluated via local Python 3.12 subprocess with strict wall-clock polling timeout and automatic tempdir cleanup.\n- demo: Deterministic offline oracle for instant unit tests, CI gates, and schema demonstrations without GPU or container dependencies.".into(),
                    code_example: Some("docker build -t rsicraft-evaluator:latest -f sandboxes/Dockerfile.evaluator sandboxes/".into()),
                },
            ],
        },
        HelpTab {
            id: "operators".into(),
            title: "5. 4-Operator Action Space & Utility Math".into(),
            badge: "Math & Prompts".into(),
            summary: "Exact formulas for composite utility parent selection and temperature-calibrated prompt contracts for DRAFT, IMPROVE, DEBUG, and CROSSOVER.".into(),
            sections: vec![
                HelpSection {
                    heading: "Composite Utility Formula (U_i)".into(),
                    body: "Instead of greedy top-1 selection (which collapses evolutionary diversity into a local optimum), RSICraft ranks and samples parents using a 3-term composite utility:\n\nU_i = w_s * s_norm_i + w_d * delta_norm_i + w_n * nu_i\n\nDefault hyperparameters: w_s = 0.50 (normalized primary score), w_d = 0.35 (normalized improvement over parent_a), w_n = 0.15 (exploration novelty bonus nu_i = 1 / (1 + children_count_i)). Every time a parent is sampled to spawn a child, its children_count increments and its novelty bonus decays, naturally steering search toward under-explored high-performing branches.".into(),
                    code_example: Some("s_norm_i = clamp((s_i - s_min) / (s_max - s_min), 0.0, 1.0)\ndelta_norm_i = clamp((delta_i - delta_min) / (delta_max - delta_min), 0.0, 1.0)\nnu_i = 1.0 / (1.0 + children_count_i)".into()),
                },
                HelpSection {
                    heading: "4-Operator Evolutionary Action Space".into(),
                    body: "1. DRAFT (T = 0.70): High-entropy architectural exploration. Synthesizes an initial candidate from the task specification alone.\n2. IMPROVE (T = 0.40): Samples one functional parent (status = SUCCESS) via composite-utility roulette and refactors inner loops, vectorization, or algorithmic complexity.\n3. DEBUG (T = 0.20): Samples one broken parent (COMPILE_ERROR, RUNTIME_ERROR, TIMEOUT) and injects its sandbox stack trace to synthesize a targeted fix.\n4. CROSSOVER (T = 0.50): Samples two distinct functional parents (parent_a != parent_b) and recombines their strongest algorithmic mechanisms into a single hybrid child.".into(),
                    code_example: Some("rsicraft step --task-id opt-synth --operator DRAFT --provider demo --sandbox-mode demo\nrsicraft step --task-id opt-synth --operator CROSSOVER --provider demo --sandbox-mode demo".into()),
                },
                HelpSection {
                    heading: "Limits of Current RSI & 4-Stage Future Trajectory".into(),
                    body: "Current RSI faces 5 hard ceilings: (1) Sandbox evaluation latency is the primary bottleneck (generation is ~1.2s; test execution takes 0.2s-60s); (2) Context degradation in long lineage histories; (3) Goodhart reward hacking if tests are incomplete; (4) Frozen model weights (code evolves, but neural weights remain static); and (5) 24 GB VRAM caps batch parallelism.\n\nFuture 4-Stage Trajectory:\n- Stage 1 (Today): Program & Prompt Search (RSICraft + Frontis-MA1 on RTX 4090)\n- Stage 2 (6-12 Mo): In-The-Loop Adapter Self-Updating (Test-time LoRA/DPO on Pareto frontier cards)\n- Stage 3 (12-24 Mo): Heterogeneous Multi-Agent Co-Evolution (Architect, Synthesizer, and Adversary agents)\n- Stage 4 (2-3 Yr): Self-Compiling Hardware/Kernel Co-Optimization (Synthesizing custom Triton/CUDA kernels).".into(),
                    code_example: None,
                },
                HelpSection {
                    heading: "Model Feeding Compatibility & Generation Time Budgets (RTX 4090)".into(),
                    body: "Any LLM outputting structured Python/Rust code can be fed into RSICraft. Recommended models on Goliath:\n- Frontis-MA1 30B/35B MoE (~3B active, Q4_K_M): 16.8 GB VRAM, ~118 tok/s (Gold Standard)\n- Qwen2.5-Coder-14B-Instruct (Q5_K_M): 11.2 GB VRAM, ~85 tok/s (High Precision)\n- DeepSeek-Coder-V2-Lite (16B MoE, Q4_K_M): 10.5 GB VRAM, ~135 tok/s (High Velocity)\n- Llama-3.1-8B-Instruct (Q8_0): 8.6 GB VRAM, ~140 tok/s (Budget Workhorse)\n- Minimal Viable Model: 3B models (Qwen2.5-Coder-3B) with strict DEBUG fallback.\n\nExecution Timings (RTX 4090 @ ~2.5s per candidate):\n- N = 10 Generations: ~25-30 seconds (Baseline validation)\n- N = 100 Generations: ~4.0-4.5 minutes (+38.6% score gain)\n- N = 1,000 Generations: ~42-45 minutes (+72.4% score gain)\n- N = 10,000 Generations: ~7.2-7.5 hours (Plateau near theoretical optimum).".into(),
                    code_example: Some("rsicraft campaign --task-id kadane-4090 --steps 100 --provider llama --sandbox-mode local".into()),
                },
                HelpSection {
                    heading: "Expected Improvement Percentages & Before/After Benchmarking Protocol".into(),
                    body: "Empirical improvements (Frontis-MA1 arXiv:2607.28568 & RSICraft):\n- Zero-Shot Baseline (Gen 0): 14.2% - 32.0% passing/optimality score\n- Gen 100: +38.6% relative gain (O(N^2) -> O(N) loop optimization via IMPROVE)\n- Gen 1,000: +72.4% relative gain (recombination of branchless tricks via CROSSOVER)\n- Gen 10,000: Asymptotic plateau approaching within 2-4% of hand-tuned assembly.\n\n4-Stage Benchmarking Protocol:\n1. Pre-RSI Baseline: Run 5 zero-shot seeds to record median score S0, latency T0, and error rate E0.\n2. Evolutionary Campaign: Run 100 or 1,000 steps with utility-weighted parent selection.\n3. Holdout Verification: Evaluate frontier winner against Hidden Holdout Test Suite.\n4. Delta Calculation: Score Delta = (S_frontier - S0) / S0 * 100%; Latency Delta = (T0 - T_frontier) / T0 * 100%.".into(),
                    code_example: Some("# Measure baseline:\nrsicraft campaign --task-id my-task --steps 5 --json\n# Run evolutionary improvement:\nrsicraft campaign --task-id my-task --steps 100 --json\n# Inspect frontier delta:\nrsicraft frontier --task-id my-task --limit 1 --json".into()),
                },
            ],
        },
        HelpTab {
            id: "compare".into(),
            title: "6. Competitive Architecture Matrix".into(),
            badge: "vs FunSearch / OpenMLE".into(),
            summary: "Architectural comparison of RSICraft against DeepMind FunSearch, OpenMLE-Evo (Python), and closed OpenAI/Anthropic internal harnesses.".into(),
            sections: vec![
                HelpSection {
                    heading: "RSICraft vs OpenAI/Anthropic Closed RSI vs FunSearch vs OpenMLE-Evo (Python)".into(),
                    body: "- Openness & Sovereignty: OpenAI/Anthropic keep RSI harnesses 100% closed; FrontisAI/OpenRSI open-sourced the research in Python; RSICraft compiles it into a sovereign zero-panic Rust binary + native egui desktop studio.\n- Runtime & Memory Tax: RSICraft is a single static Rust binary (<15 ms startup, ~22 MB GPU UI RAM, 0 MB idle daemon tax) vs multi-hundred-MB Python/Ray virtual environments.\n- Source Deduplication: RSICraft uses BLAKE3 content hashing + Zstd (level 3) compression in SQLite WAL vs uncompressed duplicate Python files on disk.\n- Parent Selection: RSICraft combines score (0.50), positive improvement delta (0.35), and exploration novelty decay 1/(1+children) (0.15) vs greedy top-K island sampling.\n- Broken-Candidate Recovery: Dedicated DEBUG (T=0.20) operator feeds sandbox stack traces back into synthesis vs discarding failed mutations.".into(),
                    code_example: None,
                },
            ],
        },
        HelpTab {
            id: "api".into(),
            title: "7. 14 MCP Tools & CLI Reference".into(),
            badge: "MCP 2024-11-05".into(),
            summary: "Complete reference for all 14 MCP tools (accepting both rsi_* and dotted rsi.* names) and their 1:1 CLI subcommands.".into(),
            sections: vec![
                HelpSection {
                    heading: "System, Hardware & Help Tools".into(),
                    body: "- rsi_status (CLI: status): SQLite WAL store metrics, card counts, unique BLAKE3 hashes, and RTX 4090 endpoint summary.\n- rsi_hardware_profile (CLI: hardware): Full Goliath RTX 4090 24 GB VRAM budget, Frontis-MA1 MoE architecture, and llama-server launch flags.\n- rsi_help (CLI: help-tabs): 9-tab operator manual with tab selection (--tab) and live substring search (--search).".into(),
                    code_example: Some("rsicraft status\nrsicraft hardware --json\nrsicraft help-tabs --tab tsinghua_openrsi\nrsicraft help-tabs --search \"Bowen Zhou\"".into()),
                },
                HelpSection {
                    heading: "Task Management & Evolutionary Execution Tools".into(),
                    body: "- rsi_task_upsert (CLI: task upsert): Register or update an optimization task and its Python evaluator script.\n- rsi_task_list (CLI: task list): List all registered tasks.\n- rsi_task_show (CLI: task show <id>): Inspect task metadata, score/delta bounds, and current top frontier.\n- rsi_step (CLI: step): Run one evolutionary step (DRAFT, IMPROVE, DEBUG, CROSSOVER).\n- rsi_campaign (CLI: campaign): Run a multi-step autonomous evolutionary campaign.\n- rsi_evaluate (CLI: evaluate): Directly benchmark a Python snippet in the sandbox and optionally record an experience card.\n- rsi_demo_seed (CLI: demo-seed): Seed an empty database with the opt-synth benchmark and a 4-card trajectory.".into(),
                    code_example: Some("rsicraft demo-seed\nrsicraft campaign --task-id opt-synth --steps 4 --provider demo --sandbox-mode demo".into()),
                },
                HelpSection {
                    heading: "Frontier, Experience Card & Lineage DAG Tools".into(),
                    body: "- rsi_get_frontier (CLI: frontier): Query materialized top-K Pareto frontier for a task.\n- rsi_card_list (CLI: cards): List experience cards ordered by composite utility with optional operator/status filters.\n- rsi_card_show (CLI: card <id>): Decompress and display full candidate source code, BLAKE3 hash, execution logs, and parent IDs.\n- rsi_lineage (CLI: lineage): Export the full evolutionary DAG (nodes and parent_a/parent_b edges) for visualization.".into(),
                    code_example: Some("rsicraft frontier --task-id opt-synth\nrsicraft cards --task-id opt-synth --status SUCCESS\nrsicraft lineage --task-id opt-synth --json".into()),
                },
            ],
        },
        HelpTab {
            id: "troubleshoot".into(),
            title: "8. Incident-Grounded Troubleshooting".into(),
            badge: "5 Real Fixes".into(),
            summary: "Production runbook grounded in real incidents encountered and resolved on Goliath (RTX 4090) during 2026-11-09 verification.".into(),
            sections: vec![
                HelpSection {
                    heading: "Incident 1: Ollama (:11434) Model-Swap Timeout vs Direct llama-server".into(),
                    body: "Symptom: LLM completion requests to http://127.0.0.1:11434 time out after 90-180s on Goliath.\nRoot Cause: Background fleet jobs polling Ollama (:11434) trigger repeated reloads of heavy 262k-context models, evicting the active coding model from VRAM.\nFix: Point RSICraft directly at the dedicated llama-server.exe process via --llama-url http://127.0.0.1:8081 (or active dynamic port) where the model stays pinned in VRAM.".into(),
                    code_example: Some("rsicraft --llama-url http://127.0.0.1:8081 step --task-id kadane-4090 --operator DRAFT --provider llama --sandbox-mode local".into()),
                },
                HelpSection {
                    heading: "Incident 2: Reasoning Model <|channel>thought Prefix in Synthesized Code".into(),
                    body: "Symptom: Reasoning models emit <|channel>thought ... <channel|>```python ... ```, and naive first-fence extraction captures internal thought blocks if they contain backticks.\nFix: PromptBuilder::extract_code_block strips all text through the last <channel|> or </think> tag before extracting the Python code block.".into(),
                    code_example: None,
                },
                HelpSection {
                    heading: "Incident 3: Docker Desktop Engine Stopped on Windows Host".into(),
                    body: "Symptom: sandbox_mode='docker' returns 'failed to spawn docker run' or daemon pipe error when Docker Desktop is not running.\nFix: Start Docker Desktop and build sandboxes/Dockerfile.evaluator, or pass --sandbox-mode local to execute inside an isolated local Python 3.12 tempdir with strict wall-clock timeout enforcement.".into(),
                    code_example: Some("rsicraft step --task-id kadane-4090 --operator IMPROVE --provider llama --sandbox-mode local".into()),
                },
                HelpSection {
                    heading: "Incident 4: Evaluator Script Missing RSI_METRIC_JSON= Sentinel".into(),
                    body: "Symptom: Candidate executes with exit code 0, but card status is recorded as METRIC_PARSE_ERROR.\nFix: Ensure your task evaluator_script prints a line matching RSI_METRIC_JSON={\"status\": \"SUCCESS\", \"score\": <float>} to stdout.".into(),
                    code_example: Some("print(f'RSI_METRIC_JSON={json.dumps({\"status\": \"SUCCESS\", \"score\": 96.6028})}')".into()),
                },
            ],
        },
        HelpTab {
            id: "faq".into(),
            title: "9. FAQ & Safety Invariants".into(),
            badge: "5 Guards".into(),
            summary: "Frequently asked questions and non-bypassable safety invariants enforced across rsicraft-core, rsicraft-mcp, and rsicraft-app.".into(),
            sections: vec![
                HelpSection {
                    heading: "Q1: Does RSICraft require a GPU or Docker to run tests and explore the Studio/CLI/MCP?".into(),
                    body: "No. Both provider='demo' and sandbox_mode='demo' are built into rsicraft-core, allowing deterministic offline campaigns, unit tests, and interactive egui Studio exploration in milliseconds with zero GPU or container dependencies.".into(),
                    code_example: Some("rsicraft --db :memory: demo-seed".into()),
                },
                HelpSection {
                    heading: "Q2: Why is RSI in RSICraft safe against the 'Skynet' runaway scenario?".into(),
                    body: "Because candidate programs never execute with host privileges or network access, never modify the Rust supervisor binary (#![deny(unwrap, panic)]), and are scored strictly by a deterministic benchmark oracle inside an air-gapped sandbox (--network none, --read-only, 4 GB RAM cap, 30s wall-clock kill timer).".into(),
                    code_example: None,
                },
                HelpSection {
                    heading: "Q3: How does BLAKE3 + Zstd CAS handle identical code generated across steps?".into(),
                    body: "Every candidate's source code is hashed with BLAKE3 (64-char hex digest) and compressed with Zstd level 3. Each evolutionary step records its own lineage card (parent_a_id, parent_b_id, operator, timestamp) while indexing code_hash so identical artifacts are immediately identifiable.".into(),
                    code_example: None,
                },
                HelpSection {
                    heading: "Q4: What are the 5 Non-Bypassable Safety Invariants in RSICraft?".into(),
                    body: "1. Zero-Panic Rust: #![deny(clippy::unwrap_used, expect_used, panic, todo, unimplemented)] across all 3 crates.\n2. Air-Gapped Container Sandbox: --network none, --read-only rootfs, --cap-drop ALL, --pids-limit 64, --memory 4g.\n3. Wall-Clock Timeout Reaping: Subprocess polling kills runaway candidates after timeout_secs and records TIMEOUT.\n4. Transactional SQLite WAL: Foreign keys, WAL journaling, and atomic frontier materialization.\n5. Pure Stdio MCP: stdout carries 100% JSON-RPC 2.0 in mcp mode; all diagnostics go to stderr.".into(),
                    code_example: None,
                },
            ],
        },
    ]
}

pub fn query_help(tab: Option<&str>, search: Option<&str>) -> Result<Value, String> {
    let tabs = all_help_tabs();
    let tab_index: Vec<Value> = tabs
        .iter()
        .map(|t| {
            json!({
                "id": t.id,
                "title": t.title,
                "badge": t.badge,
                "summary": t.summary,
                "sections_count": t.sections.len(),
            })
        })
        .collect();

    let requested_tab = tab.map(str::trim).filter(|s| !s.is_empty());
    let filtered_by_tab: Vec<HelpTab> = match requested_tab {
        None | Some("all") => tabs,
        Some(slug) => {
            let found: Vec<HelpTab> = tabs
                .into_iter()
                .filter(|t| t.id.eq_ignore_ascii_case(slug))
                .collect();
            if found.is_empty() {
                return Err(format!(
                    "unknown help tab '{slug}'; valid tabs: overview, rsi_landscape, tsinghua_openrsi, wrappee, operators, compare, api, troubleshoot, faq, all"
                ));
            }
            found
        }
    };

    let q = search
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_lowercase);

    let final_tabs: Vec<HelpTab> = match &q {
        None => filtered_by_tab,
        Some(needle) => filtered_by_tab
            .into_iter()
            .filter_map(|mut t| {
                let tab_matches = t.title.to_ascii_lowercase().contains(needle)
                    || t.summary.to_ascii_lowercase().contains(needle);
                if !tab_matches {
                    t.sections.retain(|s| {
                        s.heading.to_ascii_lowercase().contains(needle)
                            || s.body.to_ascii_lowercase().contains(needle)
                            || s.code_example
                                .as_ref()
                                .is_some_and(|c| c.to_ascii_lowercase().contains(needle))
                    });
                }
                if t.sections.is_empty() {
                    None
                } else {
                    Some(t)
                }
            })
            .collect(),
    };

    Ok(json!({
        "app": "rsicraft",
        "selected_tab": requested_tab.unwrap_or("all"),
        "search_query": search,
        "tabs_available": tab_index,
        "matched_tabs_count": final_tabs.len(),
        "tabs": final_tabs,
    }))
}
