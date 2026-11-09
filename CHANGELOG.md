# Changelog

## Unreleased

- README: four-panel cartoon (`docs/cartoon/rsi-cartoon.svg`): a tiny kitten-like LLM goes into the RSI loop on Goliath; at generation 4096 it is still tiny, its shadow is not.

## 0.1.0 - 2026-11-09

- Initial 3-crate Pure-Rust ArtCraft workspace (`rsicraft-core`, `rsicraft-mcp`, `rsicraft-app`) implementing the RSICraft Technical Implementation Specification for Goliath (RTX 4090 24 GB VRAM + Tsinghua/Frontis.AI `Frontis-MA1` 30B/35B MoE, `arXiv:2607.28568`, `FrontisAI/OpenRSI`).
- `rsicraft-core` (`L1 Core + Store`): Stigmergic SQLite WAL database (`tasks`, `experience_cards`, `task_frontiers`), BLAKE3 + Zstd Content-Addressable Storage, composite utility parent selector (`U_i = 0.50*s_norm + 0.35*delta_norm + 0.15*nu_i`), 4-operator evolutionary loop (`DRAFT`, `IMPROVE`, `DEBUG`, `CROSSOVER`), `llama-server` client with reasoning-tag stripping, isolated Docker / local Python 3.12 / deterministic `demo` sandbox runner, and 9-tab operator manual (`rsi_help`).
- `rsicraft-mcp` (`L2 CLI + MCP`): Single binary exposing 14 MCP `2024-11-05` stdio tools and 1:1 matching CLI subcommands (`--json` + formatted table/help renderer).
- `rsicraft-app` (`L3 Native GPU Desktop Studio`): `eframe`/`egui 0.37` desktop workstation with high-contrast `slate-950` dark & `slate-50` light themes, vector Sun/Moon toggle, `Ctrl+MouseWheel` zoom, compile-time Tofu Glyph Guard (`glyph_allowlist_no_tofu_chars`), Cubic Bezier Lineage DAG Canvas, Score Trajectory Spark Chart, RTX 4090 VRAM gauge, and built-in 9-tab Tsinghua/OpenRSI & Geopolitics dossier.
- Complete documentation stack (`README.md`, `INSTALL.md`, `HELP.md`, `docs/RSI_LANDSCAPE_AND_SKYNET_MYTH.md`, `docs/TSINGHUA_OPENRSI_TEAM.md`, `docs/PRD.md`, `docs/ARCHITECTURE.md`, `docs/TOOLS.md`, `docs/CONFIGURATION.md`, `docs/DEVELOPMENT.md`, `docs/TROUBLESHOOTING.md`) and compiled `BookCraft` brochure suite (`docs/brochure/rsicraft-brochure.{html,epub,pdf}`).
