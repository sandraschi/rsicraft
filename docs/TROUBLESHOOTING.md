# Incident-Grounded Troubleshooting Runbook: RSICraft

**Navigation:** [README](../README.md) · [Install](../INSTALL.md) · [Development](DEVELOPMENT.md)

This runbook documents real failure modes encountered and resolved during live verification on **Goliath** (AMD Ryzen 9 7900X, 64 GB DDR5, NVIDIA GeForce RTX 4090 24 GB VRAM) on **2026-11-09**.

---

## 1. Ollama (`:11434`) Model-Swap Timeout vs. Direct `llama-server.exe`

- **Symptom:** Running `rsicraft step --provider llama` against `http://127.0.0.1:11434` times out after 90–180 seconds.
- **Root Cause:** Background processes polling Ollama (`:11434`) request heavy 262k-context models (`gemma4:e2b`), causing Ollama to unload and reload 15–20 GB GGUF weights on every request.
- **Fix:** Connect `rsicraft` directly to the dedicated `llama-server.exe` port (`--llama-url http://127.0.0.1:8081` or the active runner port discovered via `Get-NetTCPConnection -OwningProcess <PID>`).

---

## 2. Reasoning Models Emitting `<|channel>thought` or `<think>` Preambles

- **Symptom:** Reasoning models (Gemma-4, Qwen3, DeepSeek-R1) emit `<|channel>thought ... <channel|>` before the final ````python` block. If the thought block contains markdown fences, naive extraction grabs unfinished scratch code.
- **Fix:** Fixed in `PromptBuilder::extract_code_block` (`crates/rsicraft-core/src/operators.rs`), which strips all text through the last `<channel|>` or `</think>` delimiter before extracting the Python block.

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
