# Kibitz & Architectural Review Notes (`KIBITZ.md`)

## 2026-11-09 — Architectural Audit & Live RTX 4090 Verification

- **3-Crate ArtCraft Alignment (`rsicraft-core`, `rsicraft-mcp`, `rsicraft-app`):**
  - Consolidated the Stigmergic SQLite `WAL` store and BLAKE3+Zstd Content-Addressable Storage into `rsicraft_core::store`, keeping the workspace strictly at 3 crates (`L1` Core+Store, `L2` CLI+MCP, `L3` Native `eframe`/`egui 0.37` GPU Desktop Studio).
- **Live Goliath RTX 4090 Findings:**
  - Direct connection to `llama-server.exe` avoids Ollama `:11434` model-swap evictions.
  - Reasoning models emit `<|channel>thought...<channel|>` preambles; `PromptBuilder::extract_code_block` strips thought blocks prior to fence extraction.
  - On the 25,000-element subarray benchmark (`kadane-4090`), `CROSSOVER` recombined two working parents by replacing inner-loop `max()` function calls with inline conditional branches, improving execution time from `1.391 ms` (`96.5207`) to `1.359 ms` (`96.6028`).
