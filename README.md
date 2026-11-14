# rsicraft

## Install

Tell Cursor, Claude, Antigravity, or OpenCode: **Install this.**

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-craft.ps1
```

Installs the Windows app, the CLI, and the MCP bundle.

rsicraft lets a language model running on your own machine write, test and improve small programs in a loop, and records every attempt with its parents. It is a single Rust program with a desktop studio, a command line and an MCP server, built to test the "self-improvement ladder" of the Frontis-MA1 paper on one consumer GPU.

**New here?** Read [docs/GUIDE.md](docs/GUIDE.md) (or the Help page in the studio, or `rsicraft help-tabs`). It explains what this is, what recursive self-improvement means, what the project is based on, how to connect Ollama or LM Studio, how to write a task, and every error message.

## What it does

You give rsicraft a **task**: a description such as "write `solve(n)` that counts the primes below n, as fast as possible", and an **evaluator**, a small script that runs a candidate and prints a score.

rsicraft asks a model (Ollama, LM Studio, llama-server or any OpenAI-compatible server) to write a candidate, runs it in a sandbox, and stores the result as an **experience card**. Then it repeats with four operators: DRAFT (new), IMPROVE (make a working parent better), DEBUG (repair a failed one) and CROSSOVER (combine two). Parents are chosen by score, gain over their own parent, and novelty. You can follow the lineage of the best solution step by step.

## What it is not

The model does not change during a search; only the archive of programs grows. Training a next model generation from search results (rung 2 of the ladder) has tools in this repository (trace export, a LoRA harness, an A/B promotion gate) but has not been run, and no improvement from it has been measured. See [What is RSI?](docs/GUIDE.md#2-what-is-rsi) and the dated status in [docs/FRONTIS_PARITY_PLAN.md](docs/FRONTIS_PARITY_PLAN.md).

## Status (2026-11-14)

- The search loop works with live local models, in local and Docker sandboxes.
- Benchmark pack-v1 (six algorithm tasks, 35B models): Frontis-MA1-35B showed no advantage over its base model, and search added little (median amplification 1.18). [docs/BENCH_MODELSWAP_V2.md](docs/BENCH_MODELSWAP_V2.md)
- Benchmark pack-v2 (9B model, finished 2026-11-12): 13 of 15 runs have zero success cards. The amplification gate was not met. The recorded failures are the completion budget and Ollama HTTP 500. [docs/BENCH_AMP_V2.md](docs/BENCH_AMP_V2.md)
- Model server failures are classified and reported with a fix; campaigns check the server first and stop on setup problems.

## Quickstart

```powershell
# Build
cargo build --release

# 1. Try it without a model (scores are invented in demo mode)
.\target\release\rsicraft --db demo.db demo-seed
.\target\release\rsicraft --db demo.db campaign --task-id opt-synth --steps 4 --provider demo --sandbox-mode demo
.\target\release\rsicraft --db demo.db lineage --task-id opt-synth

# 2. Connect a model and check it (must say OK)
$env:RSICRAFT_LLM_URL = 'http://127.0.0.1:11434'   # Ollama; LM Studio is :1234
$env:RSICRAFT_MODEL   = 'qwen2.5-coder:7b'
.\target\release\rsicraft probe

# 3. A real search
.\target\release\rsicraft task upsert --task-id primes --name "Prime count" `
  --description "Write solve(n) returning the number of primes below n, as fast as possible. Standard library only." `
  --evaluator-file tasks\pack-v1\primes.py
.\target\release\rsicraft campaign --task-id primes --steps 8 --provider llama --sandbox-mode local
.\target\release\rsicraft frontier --task-id primes

# 4. The desktop studio
.\target\release\rsicraft-app
```

The local sandbox runs model-written code as you. For anything you would not run yourself, build the Docker image and use `--sandbox-mode docker` (no network, read-only, no privileges, memory and CPU limits). Details: [Quickstart](docs/GUIDE.md#4-quickstart), [Connecting a model](docs/GUIDE.md#5-connecting-a-model).

## When something fails

Every model server problem is printed as `[llm:<class>] what happened | fix: what to do`, for example:

```text
rsicraft: [llm:model_not_found] Ollama at http://127.0.0.1:11434 does not serve 'qwen2.5-coder:7b'; served models: ... | fix: ... `ollama list`, then `ollama pull <name>` ...
```

`rsicraft probe` checks the server, lists its models, warns about context windows that are too small, and finds servers on the usual local ports when nothing answers at the configured address. Campaigns stop at once on setup problems (unreachable, unknown model, missing API key, bad address); per-step problems are recorded with their class. The studio shows the last problem and its fix in the Live Run panel. All classes: [Troubleshooting](docs/GUIDE.md#9-troubleshooting).

## What it is based on

- **Frontis-MA1 / OpenRSI** (arXiv:2607.28568, July 2026; Horizon Research, Frontis.AI, Tsinghua University; [github.com/FrontisAI/OpenRSI](https://github.com/FrontisAI/OpenRSI)): trained Draft, Improve, Debug and Crossover operators and a long-horizon search harness for machine-learning engineering. rsicraft reimplements the operator set, temperatures, experience cards and utility-based parent selection in Rust. It does not contain OpenRSI code (that code is CC BY-NC 4.0; rsicraft is MIT or Apache-2.0).
- **LLM-guided program search**: FunSearch (2023) and AlphaEvolve (2025) from Google DeepMind, AIDE and similar agents.

More, including what the paper does and does not claim: [What this is based on](docs/GUIDE.md#3-what-this-is-based-on).

## Three ways to use it

| | Start with | For |
| :--- | :--- | :--- |
| Desktop studio | `rsicraft-app` | Watching runs, the lineage graph, code and diffs of every candidate |
| Command line | `rsicraft --help`, `rsicraft help-tabs` | Scripts and benchmarks; every verb accepts `--json` |
| MCP server | `rsicraft mcp` | Letting Claude or another agent drive searches (26 tools) |

All three use the same engine and the same SQLite database (`%LOCALAPPDATA%\rsicraft\rsicraft.db` by default, or `--db` / `RSICRAFT_DB`).

## Repository layout

```text
crates/rsicraft-core   engine: store, operators, selection, LLM client, sandboxes, help text
crates/rsicraft-mcp    the rsicraft binary: CLI and MCP stdio server
crates/rsicraft-app    the rsicraft-app desktop studio (egui)
sandboxes/             Docker evaluator image, LoRA training harness
tasks/                 task packs (pack-v1 algorithms, pack-v2 heuristics)
scripts/               benchmark runner and analysis, help document generator
docs/                  guide, plans, benchmark reports, configuration, troubleshooting
```

## Documentation

| Document | Contents |
| :--- | :--- |
| [docs/GUIDE.md](docs/GUIDE.md) | The manual, generated from the built-in help |
| [docs/help/index.html](docs/help/index.html) | The same, as a searchable page |
| [INSTALL.md](INSTALL.md) | Installation and MCP client setup |
| [docs/CONFIGURATION.md](docs/CONFIGURATION.md) | Environment variables and flags |
| [docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md) | Error classes and incident notes |
| [docs/TOOLS.md](docs/TOOLS.md) | MCP tool schemas |
| [docs/FRONTIS_PARITY_PLAN.md](docs/FRONTIS_PARITY_PLAN.md) | Ladder status, plan, gates |
| [docs/PRD.md](docs/PRD.md), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) | Requirements, design, build and release |
| [docs/BENCH_MODELSWAP_V2.md](docs/BENCH_MODELSWAP_V2.md), [docs/BENCH_AMP_V2.md](docs/BENCH_AMP_V2.md) | Benchmark reports |

Opinion and outreach pieces, not documentation: [docs/BOSTROM_YUDKOWSKY_AND_RSI_REALITY.md](docs/BOSTROM_YUDKOWSKY_AND_RSI_REALITY.md), [docs/RSI_LANDSCAPE_AND_SKYNET_MYTH.md](docs/RSI_LANDSCAPE_AND_SKYNET_MYTH.md), [docs/TSINGHUA_OPENRSI_TEAM.md](docs/TSINGHUA_OPENRSI_TEAM.md), [docs/LITTLE_TIMMY_EXPLAINS_RSI.md](docs/LITTLE_TIMMY_EXPLAINS_RSI.md) (cartoon), [docs/brochure/](docs/brochure/). Some of their numbers are illustrative, not measured.

## License

MIT or Apache-2.0, at your option. The Frontis-MA1 model weights, if you use them, are under their own CC BY-NC 4.0 licence.
