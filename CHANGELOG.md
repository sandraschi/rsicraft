# Changelog

## 0.1.0 - 2026-11-09

- Initial 3-crate Pure-Rust ArtCraft workspace (`rsicraft-store`, `rsicraft-core`, `rsicraft-mcp`) implementing the RSICraft Technical Implementation Specification for Goliath (RTX 4090 24 GB VRAM + Frontis-MA1 30B/35B MoE).
- Stigmergic SQLite WAL database (`tasks`, `experience_cards`, `task_frontiers`), BLAKE3 + Zstd Content-Addressable Storage, and composite utility parent selector (`U_i = 0.50*s_norm + 0.35*delta_norm + 0.15*nu_i`).
- 4-operator evolutionary loop (`DRAFT`, `IMPROVE`, `DEBUG`, `CROSSOVER`) with operator-specific temperatures, `llama-server` client, isolated Docker evaluator (`sandboxes/Dockerfile.evaluator`), isolated local Python 3.12 evaluator (`sandbox_mode = "local"`), deterministic `demo` oracle, CLI (`--json`), and MCP `2024-11-05` stdio server (13 tools).
- Verified live 4-step evolutionary campaign on Goliath RTX 4090 (`llama-server` with speculative decoding + Python 3.12 sandbox on 25,000-element subarray benchmark: `DRAFT` 96.5207 -> `CROSSOVER` 96.6028 at 1.359 ms by replacing inner-loop `max()` calls with inline branching).
