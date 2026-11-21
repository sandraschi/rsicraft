# rsicraft

rsicraft is a local-first evolutionary program search studio that lets an LLM on your machine write, evaluate, repair, and improve small programs in a closed loop while tracking every attempt's lineage.

## What You Can Do

- **Run evolutionary code search (Rung 1)**: Define a task and an evaluator script, then let a local or OpenAI-compatible model iterate using four operators:
  - `DRAFT` — synthesize a fresh candidate from the task specification.
  - `IMPROVE` — mutate a high-utility working parent to optimize score or runtime.
  - `DEBUG` — repair a failed parent (`COMPILE_ERROR`, `RUNTIME_ERROR`, `TIMEOUT`, `METRIC_PARSE_ERROR`) using its sandbox log tail.
  - `CROSSOVER` — combine ideas from two distinct high-utility parents into a stronger child.
- **Inspect candidate lineage in the Desktop Studio**: Watch live runs, compare candidate code and diffs side by side, and explore the interactive Bezier lineage DAG across 7 views in `rsicraft-app`.
- **Chat and get help in-app**: Press `Ctrl+J` to open the **AI Chat Dock**, press `Ctrl+K` for the Command Palette, or open the **10-item Help menu** (including **Bundled Skills** and the built-in 10-tab manual).
- **Use bundled MCP skills and 26 tools**: Connect Claude Desktop or any MCP client via `rsicraft mcp` to drive searches, inspect frontiers, and read `skill://` resources.
- **Run safely in local or Docker sandboxes**: Execute model-written candidates locally for speed or isolate untrusted code in `--sandbox-mode docker` (no network, read-only root, CPU and memory limits).
- **Persist everything offline**: Store all tasks, experience cards, and execution logs in a local SQLite WAL database (`%LOCALAPPDATA%\rsicraft\rsicraft.db`) backed by a BLAKE3 + Zstd content-addressable store.
- **Connect any local LLM server**: Works with Ollama (`:11434`), LM Studio (`:1234`), `llama-server`, or any OpenAI-compatible endpoint, with structured `[llm:<class>]` diagnostics via `rsicraft probe`.
- **Export Rung 2 training traces**: Export SFT/DPO datasets from search archives (`rsi_dataset_export`) and run the LoRA / A/B promotion gate harness (experimental; weights do not update during search).

## Install

1. Download **`rsicraft-0.3.1-setup.exe`** from the [latest release](https://github.com/sandraschi/rsicraft/releases/latest).
2. Run it. Windows may show a SmartScreen warning because the installer is unsigned: choose **More info > Run anyway**.
3. Open **rsicraft** from the Start menu.

**Only want the tools inside Claude?** The `.mcpb` bundle registers the MCP server in Claude Desktop but does **not** install the desktop studio. Paste this to Claude:

> Install the rsicraft MCP extension: download `rsicraft-0.3.1.mcpb` from https://github.com/sandraschi/rsicraft/releases/latest and open it with Claude Desktop.

**From a local checkout (Windows):**

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\install-craft.ps1
```

Installs the Windows app and the CLI (Start Menu shortcut, CLI on the user PATH). `-RegisterClients` also registers the MCP exe with the IDEs that are installed. `-Clients cursor,claude-desktop` limits that list. `-Mcpb` installs the Claude Desktop extension bundle. See [INSTALL.md](INSTALL.md) for manual MCP client config and [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for building from source.

## Example Prompts

> "Seed the demo database and run a 4-step search campaign on `opt-synth`, then show the lineage."

> "Probe my local Ollama server on port 11434 and check if `qwen2.5-coder:7b` is ready."

> "Create a `primes` task using `tasks/pack-v1/primes.py` and run 8 search steps in the local sandbox."

## Three Ways to Use It

| Surface | Start With | Best For |
|---|---|---|
| **Desktop Studio** | `rsicraft-app` | Watching live runs, lineage graphs, candidate code, and diffs |
| **Command Line** | `rsicraft --help` | Scripts and benchmarks; every verb supports `--json` |
| **MCP Server** | `rsicraft mcp` | Driving searches from Claude Desktop or agentic IDEs (26 tools) |

## Keyboard Shortcuts (Desktop Studio)

| Shortcut | Action |
|---|---|
| `Ctrl+J` | Toggle the **AI Chat Dock** |
| `Ctrl+K` | Open the Command Palette |
| `F1` | Open the built-in Help Center |

## Honest Status

During a search, model weights do not change — only the archive of experience cards grows. Rung 2 training tools (`rsi_dataset_export`, `rsi_train_step`, `rsi_promote_gate`) exist in the repository, but measured model weight improvement has not yet been demonstrated. Benchmark notes are in [docs/BENCH_MODELSWAP_V2.md](docs/BENCH_MODELSWAP_V2.md) and [docs/BENCH_AMP_V2.md](docs/BENCH_AMP_V2.md).

## Documentation

| Document | Contents |
|---|---|
| [HELP.md](HELP.md) | Complete operator manual and ecosystem reference |
| [docs/GUIDE.md](docs/GUIDE.md) | User guide generated from the built-in 10-tab help |
| [docs/help/index.html](docs/help/index.html) | Searchable HTML version of the manual |
| [INSTALL.md](INSTALL.md) | All installation paths and MCP client setup |
| [docs/CONFIGURATION.md](docs/CONFIGURATION.md) | Environment variables (`RSICRAFT_LLM_URL`, `RSICRAFT_MODEL`, `RSICRAFT_DB`) |
| [docs/TOOLS.md](docs/TOOLS.md) | Full reference for all 26 MCP tools |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Crate layout, SQLite WAL + CAS storage, sandbox design |
| [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) | Building from source, tests, and release packaging |
| [docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md) | `[llm:<class>]` error codes and fixes |

## Requirements

- **OS**: Windows 10/11 (x64) for the installer; Linux/macOS supported from source
- **LLM (optional for demo mode)**: Ollama, LM Studio, or any OpenAI-compatible server
- **Docker (optional)**: Required only for `--sandbox-mode docker`

## License

MIT or Apache-2.0, at your option. Frontis-MA1 model weights, if you use them, carry their own CC BY-NC 4.0 license.
