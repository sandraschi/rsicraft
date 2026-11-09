# Changelog

## 0.2.0 - 2026-11-09

- **Expanded Tool Surface (14 to 22 Tools):** Complete 100% CLI and MCP parity closing all functional gaps:
  - `rsi_diff` (`rsicraft diff --card-a <id> --card-b <id>`): Unified line-by-line diff comparing decompressed CAS source codes between two experience cards.
  - `rsi_llm_probe` (`rsicraft probe [--url <url>]`): Active `/v1/models` reachability, latency in ms, and active model alias inspection.
  - `rsi_hyperparams_get` / `rsi_hyperparams_set` (`rsicraft hyperparams`): Dynamic runtime inspection and adjustment of composite utility weights ($\lambda_s, \lambda_\delta, \lambda_\nu$) without restarts.
  - `rsi_task_delete` (`rsicraft task delete <id>`): Clean cascading task deletion breaking self-referential foreign key cycles.
  - `rsi_cards_prune` (`rsicraft prune --task-id <id> [--keep-frontier <k>]`): Prunes low-utility non-frontier cards while preserving top Pareto performers.
  - `rsi_export` / `rsi_import` (`rsicraft export --task-id <id>`, `rsicraft import --file <path>`): Standalone JSON task bundle export and import with CAS preservation.
  - Multi-language sandbox runner: Added native Rust compilation and evaluation (`rustc --crate-type=rlib candidate.rs` + `rustc evaluator.rs`) alongside Python 3.12.
- **Desktop Studio Enhancements (`rsicraft-app`):**
  - Interactive line-by-line `[Diff with Parent]` viewer embedded directly into the Artifact Inspector.
  - Interactive 3D NVIDIA GeForce RTX 4090 GPU Studio with dual contra-rotating flow-through fans, 12x GDDR6X VRAM chips, AD102 die, and style toggles.
  - 3D Evolutionary Fitness Manifold on the Lineage DAG page with streaming particles and Pareto hypercube wireframe.
  - Updated tool count badges, help reader tables, and smoke check to 22 tools.
- **Philosophical & Engineering Treatises:**
  - `docs/BOSTROM_YUDKOWSKY_AND_RSI_REALITY.md`: Analysis of Bostrom, Yudkowsky, the Foom myth, and physical constraints refuting Skynet.
  - `docs/RSI_ENGINEERING_LIMITS_AND_BENCHMARKING.md`: 5 hard ceilings, 4-stage future trajectory, model feeding matrix, and 4-step benchmarking protocol.
- **Fleet Integration:**
  - Added root `start.bat` and `start.ps1` launchers, mirrored to `mcp-central-docs/starts/rsicraft-start.bat`.
  - Rebuilt complete technical brochure suite (`HTML`, `EPUB`, `PDF`) via `BookCraft`.

## 0.1.0 - 2026-11-09

- Initial 3-crate Pure-Rust ArtCraft workspace (`rsicraft-core`, `rsicraft-mcp`, `rsicraft-app`) implementing the RSICraft Technical Implementation Specification for Goliath (RTX 4090 24 GB VRAM + Tsinghua/Frontis.AI `Frontis-MA1` 30B/35B MoE, `arXiv:2607.28568`, `FrontisAI/OpenRSI`).
- `rsicraft-core` (`L1 Core + Store`): Stigmergic SQLite WAL database (`tasks`, `experience_cards`, `task_frontiers`), BLAKE3 + Zstd Content-Addressable Storage, composite utility parent selector (`U_i = 0.50*s_norm + 0.35*delta_norm + 0.15*nu_i`), 4-operator evolutionary loop (`DRAFT`, `IMPROVE`, `DEBUG`, `CROSSOVER`), `llama-server` client with reasoning-tag stripping, isolated Docker / local Python 3.12 / deterministic `demo` sandbox runner, and 9-tab operator manual (`rsi_help`).
- `rsicraft-mcp` (`L2 CLI + MCP`): Single binary exposing 14 MCP `2024-11-05` stdio tools and 1:1 matching CLI subcommands (`--json` + formatted table/help renderer).
- `rsicraft-app` (`L3 Native GPU Desktop Studio`): `eframe`/`egui 0.37` desktop workstation with high-contrast `slate-950` dark & `slate-50` light themes, vector Sun/Moon toggle, `Ctrl+MouseWheel` zoom, compile-time Tofu Glyph Guard (`glyph_allowlist_no_tofu_chars`), Cubic Bezier Lineage DAG Canvas, Score Trajectory Spark Chart, RTX 4090 VRAM gauge, and built-in 9-tab Tsinghua/OpenRSI & Geopolitics dossier.
- Complete documentation stack (`README.md`, `INSTALL.md`, `HELP.md`, `docs/RSI_LANDSCAPE_AND_SKYNET_MYTH.md`, `docs/TSINGHUA_OPENRSI_TEAM.md`, `docs/PRD.md`, `docs/ARCHITECTURE.md`, `docs/TOOLS.md`, `docs/CONFIGURATION.md`, `docs/DEVELOPMENT.md`, `docs/TROUBLESHOOTING.md`) and compiled `BookCraft` brochure suite (`docs/brochure/rsicraft-brochure.{html,epub,pdf}`).
