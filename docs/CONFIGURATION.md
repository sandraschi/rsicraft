# Configuration Reference: RSICraft

**Navigation:** [README](../README.md) · [Install](../INSTALL.md) · [Architecture](ARCHITECTURE.md) · [Tools](TOOLS.md)

---

## 1. Global CLI Flags & Environment Variables

| CLI Flag | Environment Variable | Default Value | Description |
| :--- | :--- | :--- | :--- |
| `--db <PATH>` | `RSICRAFT_DB` | `%LOCALAPPDATA%\rsicraft\rsicraft.db` | Path to the SQLite `WAL` database file (use `:memory:` for ephemeral runs). |
| `--llama-url <URL>` | - | `http://127.0.0.1:8081` | Base URL of `llama-server` or Ollama serving `/v1/chat/completions`. |
| `--model <ALIAS>` | - | `frontis` | Model identifier sent in the OpenAI-compatible completion payload. |
| `--docker-image <TAG>` | - | `rsicraft-evaluator:latest` | Container image tag used when `--sandbox-mode docker` is selected. |
| - | `RSICRAFT_PYTHON` | `python3` (or uv Python 3.12 on Windows) | Custom path to Python executable for local sandbox evaluation. |
| - | `RSICRAFT_RUSTC` | `rustc` (or `%USERPROFILE%\.cargo\bin\rustc.exe`) | Custom path to `rustc` compiler for local native Rust sandbox evaluation. |
| - | `RSICRAFT_MAX_TOKENS` | `4096` | Completion budget per LLM synthesis call (64 to 65536). Raise it when a reasoning model spends the budget on its thought preamble and the code block comes back truncated. |
| `--json` | - | `false` | Emit machine-readable pretty-printed JSON on `stdout`. |

---

## 2. Composite Utility Hyperparameters (`UtilityHyperparams`)

Defined in `crates/rsicraft-core/src/store/utility.rs` and dynamically tunable via `rsicraft hyperparams` / `rsi_hyperparams_set`:

| Parameter | Symbol | Default | Role |
| :--- | :---: | :---: | :--- |
| `weight_score` | $\lambda_s$ | `0.50` | Weight on min-max normalized primary score $s_{\text{norm}, i}$. |
| `weight_delta` | $\lambda_\Delta$ | `0.35` | Weight on min-max normalized score improvement over `parent_a` ($\Delta_{\text{norm}, i}$). |
| `weight_novelty` | $\lambda_\nu$ | `0.15` | Weight on exploration novelty bonus $\nu_i = \frac{1}{1 + \text{children\_count}_i}$. |

The weights are stored in the database (`settings` table). Setting them re-ranks every task immediately and the values stay in effect across CLI invocations, MCP server restarts, and the desktop studio. Tune them with:
```powershell
rsicraft hyperparams --w-score 0.60 --w-delta 0.30 --w-novelty 0.10
```

---

## 3. Sandbox Execution Modes (`sandbox_mode`)

| Mode | Isolation Mechanism | When to Use |
| :--- | :--- | :--- |
| `"demo"` | Deterministic in-memory Rust oracle | Unit tests, CI gates, offline exploration without GPU/Docker. |
| `"local"` | Ephemeral temporary directory + Python or native Rust (`rustc`) + host-side timeout reaper. **No isolation**: the candidate runs with your user rights, network and filesystem. | Fast local campaigns with code you trust (your own model on your own tasks) when Docker Desktop is stopped. |
| `"docker"` | Non-root container (`--network none --read-only --user 10000:10001 --cap-drop ALL --security-opt no-new-privileges --pids-limit 256 --memory 4g --cpus 4`). Build the image first: `docker build -f sandboxes/Dockerfile.evaluator -t rsicraft-evaluator:latest sandboxes`. | Untrusted long-horizon synthesis requiring strict kernel/network containment. |
