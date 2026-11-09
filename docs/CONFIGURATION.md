# Configuration Reference: RSICraft

**Navigation:** [README](../README.md) · [Install](../INSTALL.md) · [Architecture](ARCHITECTURE.md) · [Tools](TOOLS.md)

---

## 1. Global CLI Flags & Environment Variables

| CLI Flag | Environment Variable | Default Value | Description |
| :--- | :--- | :--- | :--- |
| `--db <PATH>` | `RSICRAFT_DB` | `%LOCALAPPDATA%\rsicraft\rsicraft.db` | Path to the SQLite `WAL` database file (use `:memory:` for ephemeral runs). |
| `--llama-url <URL>` | — | `http://127.0.0.1:8081` | Base URL of `llama-server` or Ollama serving `/v1/chat/completions`. |
| `--model <ALIAS>` | — | `frontis` | Model identifier sent in the OpenAI-compatible completion payload. |
| `--docker-image <TAG>` | — | `rsicraft-evaluator:latest` | Container image tag used when `--sandbox-mode docker` is selected. |
| — | `RSICRAFT_PYTHON` | `python3` (or uv Python 3.12 on Windows) | Custom path to Python executable for local sandbox evaluation. |
| — | `RSICRAFT_RUSTC` | `rustc` (or `%USERPROFILE%\.cargo\bin\rustc.exe`) | Custom path to `rustc` compiler for local native Rust sandbox evaluation. |
| `--json` | — | `false` | Emit machine-readable pretty-printed JSON on `stdout`. |

---

## 2. Composite Utility Hyperparameters (`UtilityHyperparams`)

Defined in `crates/rsicraft-core/src/store/utility.rs` and dynamically tunable via `rsicraft hyperparams` / `rsi_hyperparams_set`:

| Parameter | Symbol | Default | Role |
| :--- | :---: | :---: | :--- |
| `weight_score` | $\lambda_s$ | `0.50` | Weight on min-max normalized primary score $s_{\text{norm}, i}$. |
| `weight_delta` | $\lambda_\Delta$ | `0.35` | Weight on min-max normalized score improvement over `parent_a` ($\Delta_{\text{norm}, i}$). |
| `weight_novelty` | $\lambda_\nu$ | `0.15` | Weight on exploration novelty bonus $\nu_i = \frac{1}{1 + \text{children\_count}_i}$. |

Tune dynamically at runtime:
```powershell
rsicraft hyperparams --w-score 0.60 --w-delta 0.30 --w-novelty 0.10
```

---

## 3. Sandbox Execution Modes (`sandbox_mode`)

| Mode | Isolation Mechanism | When to Use |
| :--- | :--- | :--- |
| `"demo"` | Deterministic in-memory Rust oracle | Unit tests, CI gates, offline exploration without GPU/Docker. |
| `"local"` | Isolated temporary directory + Python 3.12 or native Rust (`rustc`) compilation + 50 ms timeout reaper | Fast local evolutionary campaigns on Goliath when Docker Desktop is stopped. |
| `"docker"` | Rootless container (`--network none --read-only --cap-drop ALL --memory 4g --cpus 4`) | Untrusted long-horizon synthesis requiring strict kernel/network containment. |
