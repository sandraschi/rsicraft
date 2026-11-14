# Incident-Grounded Troubleshooting Runbook: RSICraft

**Navigation:** [README](../README.md) · [Install](../INSTALL.md) · [Development](DEVELOPMENT.md)

Every model server error is printed as `[llm:<class>] what happened | fix: what to do`. The table of all classes and their fixes is in the guide: [Troubleshooting](GUIDE.md#9-troubleshooting) (also `rsicraft help-tabs --tab troubleshoot`). Run `rsicraft probe` first for any provider problem.

This runbook documents real failure modes encountered and resolved during live verification on **Goliath** (AMD Ryzen 9 7900X, 64 GB DDR5, NVIDIA GeForce RTX 4090 24 GB VRAM) on **2026-11-09**.

---

## 1. Ollama (`:11434`) Model-Swap Timeout vs. Direct `llama-server.exe`

- **Symptom:** A step against Ollama dies while another client loads a large context, or the runner returns HTTP 500 after a long generation.
- **What the current binary does:** The completion timeout is `RSICRAFT_LLM_TIMEOUT_SECS` (default 600). `rsicraft probe` reports the provider, whether the model is served, and a context window smaller than the completion budget. A campaign stops on unreachable, unknown model, missing key, or a bad address. Port 8081 on this machine is held by Docker. Ollama is `http://127.0.0.1:11434` (`RSICRAFT_LLM_URL` or `--llama-url`).
- **Measured case:** On 2026-11-12 a second client asked Ollama for `num_ctx 262144`, Ollama evicted the 9B runner, and the step returned HTTP 500. See `docs/BENCH_AMP_V2.md`.

---

## 2. Reasoning Models Emitting `<|channel>thought` or `<think>` Preambles

- **Symptom:** Reasoning models (Gemma-4, Qwen3, DeepSeek-R1) emit `<|channel>thought ... <channel|>` before the final ````python` block. If the thought block contains markdown fences, naive extraction grabs unfinished scratch code.
- **Fix:** `PromptBuilder::extract_code_block` drops text through the last `<channel|>` or `</think>` before taking a fence. A llama answer that still has no top-level function is `no_code` and is not evaluated.

---

## 3. Docker Desktop Daemon Stopped on Windows Host

- **Symptom:** `--sandbox-mode docker` returns `failed to spawn docker run` or pipe connection error.
- **Fix:** Either start Docker Desktop and build `sandboxes/Dockerfile.evaluator`, or pass `--sandbox-mode local` to evaluate inside an isolated local Python 3.12 temporary directory with 50 ms polling timeout reaping.

---

## 4. Evaluator Script Returns `METRIC_PARSE_ERROR`

- **Symptom:** Candidate script runs with exit code `0`, but the card records `execution_status = "METRIC_PARSE_ERROR"`.
- **Root Cause:** The task's `evaluator_script` did not print the required `RSI_METRIC_JSON=` sentinel line to `stdout`.
- **Fix:** Ensure your `evaluator_script` prints:
  ```python
  import json
  print(f'RSI_METRIC_JSON={json.dumps({"status": "SUCCESS", "score": float(score)})}')
  ```

---

## 5. `egui 0.37` Tofu Boxes (`□`) When Adding UI Labels

- **Symptom:** Unicode emojis or geometric symbols (`●`, `○`, `☀`, `🌙`) render as `□` in `eframe`/`egui 0.37`.
- **Fix:** `crates/rsicraft-app/src/main.rs` draws all status dots, sun/moon icons, and Bézier lineage curves using `ui.painter()` vector primitives and enforces the `glyph_allowlist_no_tofu_chars` unit test.
