# Configuration Reference: RSICraft

**Navigation:** [README](../README.md) · [Install](../INSTALL.md) · [Architecture](ARCHITECTURE.md) · [Tools](TOOLS.md) · [Frontis Parity Plan](FRONTIS_PARITY_PLAN.md)

---

## 1. Global CLI Flags & Environment Variables

| CLI Flag | Environment Variable | Default Value | Description |
| :--- | :--- | :--- | :--- |
| `--db <PATH>` | `RSICRAFT_DB` | `%LOCALAPPDATA%\rsicraft\rsicraft.db` | Path to the SQLite `WAL` database file (use `:memory:` for ephemeral runs). |
| `--llama-url <URL>` | `RSICRAFT_LLM_URL` | `http://127.0.0.1:8081` | Server root. An explicit flag wins, then the environment variable, then the default. Ollama is `http://127.0.0.1:11434`, LM Studio `http://127.0.0.1:1234`, llama-server `http://127.0.0.1:8080`. On this machine port 8081 is held by Docker. |
| `--model <ALIAS>` | `RSICRAFT_MODEL` | `frontis` | Model name as the server knows it. An explicit flag wins, then the environment variable, then the default. |
| - | `RSICRAFT_LLM_API_KEY` | unset | Bearer token when the endpoint requires one. |
| `--docker-image <TAG>` | - | `rsicraft-evaluator:latest` | Container image tag used when `--sandbox-mode docker` is selected. |
| - | `RSICRAFT_PYTHON` | `python3` (or uv Python 3.12 on Windows) | Custom path to Python executable for local sandbox evaluation. |
| - | `RSICRAFT_RUSTC` | `rustc` (or `%USERPROFILE%\.cargo\bin\rustc.exe`) | Custom path to `rustc` compiler for local native Rust sandbox evaluation. |
| - | `RSICRAFT_MAX_TOKENS` | `12288` | Completion token budget per LLM synthesis call (64 to 65,536). A generous budget prevents reasoning models from exhausting their tokens in thought preambles before emitting code blocks. |
| - | `RSICRAFT_LLM_TIMEOUT_SECS`| `600` | Wall-clock timeout in seconds for a single LLM completion (5 to 7,200). Generous 10-minute default ensures 35B-class reasoning models have adequate time for extended thinking. |
| - | `RSICRAFT_REASONING_EFFORT`| None (unset) | Optional reasoning effort hint (`"low"`, `"medium"`, `"high"`, `"none"`) passed directly to Ollama and OpenAI-compatible inference servers that support reasoning effort levels. |
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
| `"demo"` | Deterministic in-memory Rust oracle | Fast unit tests, CI gates, and offline smoke tests without GPU or Docker dependencies. |
| `"local"` | Ephemeral temporary directory + Python 3.12 or native Rust (`rustc`) + host-side watchdog reaper. **Convenience backend**: candidate runs with host user privileges. | Rapid local development and testing with models and tasks you control when Docker Desktop is stopped. |
| `"docker"` | Non-root container (`--network none --read-only --user 10000:10001 --cap-drop ALL --security-opt no-new-privileges --pids-limit 256 --memory 4g --cpus 4`). Payload delivered over `stdin`. | Production long-horizon evolutionary synthesis requiring strict kernel, filesystem, and network containment. |

---

## 4. Benchmark Task Pack (`tasks/pack-v1`) & Model-Swap Harness

RSICraft bundles a standardized algorithmic benchmark task pack in `tasks/pack-v1/` featuring noise-normalized speedup evaluators:

| Task ID | Name | Split | Description |
| :--- | :--- | :--- | :--- |
| `edit_distance` | Levenshtein Distance | `train` | Compute minimum edit distance between sequence pairs; score is speedup over naive matrix recurrence. |
| `inversions` | Inversion Count | `train` | Count inversion pairs in permutations; score is speedup over $O(N^2)$ brute-force check. |
| `lis` | Longest Increasing Subseq | `train` | Longest strictly increasing subsequence length; score is speedup over quadratic recursion. |
| `matmul` | Integer Matrix Multiply | `train` | Dense $N \times N$ integer matrix product; score is speedup over triple-nested loop reference. |
| `primes` | Prime Sieve Counting | `train` | Count prime numbers strictly less than $N$; score is speedup over naive trial division. |
| `wordfreq` | Word Frequency Top-K | `heldout` | Tokenize and extract top-K word frequencies; score is speedup over unindexed linear counter. |

### Running the Model-Swap Benchmark (`scripts/bench-models.ps1`)
Compare two models across identical tasks and trials with automatic resumption and CSV logging:
```powershell
pwsh -File scripts\bench-models.ps1 `
    -Models 'qwen3.6-35b-a3b:latest', 'hf.co/FrontisAI/Frontis-MA1-35B-GGUF:Q4_K_M' `
    -Steps 12 `
    -Trials 3 `
    -TimeoutSecs 60
```
Results are appended to `$env:LOCALAPPDATA\rsicraft\bench\<timestamp>\results.csv` recording step counts, success cards, best scores, and wall-clock seconds.
