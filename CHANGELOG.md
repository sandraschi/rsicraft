# Changelog

## 0.1.0 - 2026-11-09

- Initial 3-crate Pure-Rust ArtCraft workspace (`rsicraft-store`, `rsicraft-core`, `rsicraft-mcp`) implementing the RSICraft Technical Implementation Specification for Goliath (RTX 4090 24 GB VRAM + Frontis-MA1 30B/35B MoE).
- Stigmergic SQLite WAL database (`tasks`, `experience_cards`, `task_frontiers`), BLAKE3 + Zstd Content-Addressable Storage, and composite utility parent selector (`U_i = 0.50*s_norm + 0.35*delta_norm + 0.15*nu_i`).
- 4-operator evolutionary loop (`DRAFT`, `IMPROVE`, `DEBUG`, `CROSSOVER`) with operator-specific temperatures, `llama-server` client, isolated Docker evaluator (`sandboxes/Dockerfile.evaluator`), deterministic `demo` oracle, CLI (`--json`), and MCP `2024-11-05` stdio server (13 tools).
