# RSICraft - Tiered Execution Plan

**Parent Standard:** [`RUST_ARTCRAFT_STANDARD.md`](../../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)
**Menu Bar Standard:** [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)
**Architecture Model:** 4-Tier ArtCraft Incremental Delivery
**Status:** Tiers 1 through 4 are built. Undo and redo restore the SQLite store (up to 8 snapshots). Open Database and Save As use a file dialog. Cut, paste, and select all still have no selection.

---

## Architecture Flow

```
Tier 1: Core Engine --> Tier 2: MCP / CLI --> Tier 3: Desktop UI --> Tier 4: Menu bar
BUILT rsicraft-core     BUILT rsicraft         BUILT rsicraft-app     BUILT (File through Help)
```

---

## Tier Specifications

### Tier 1 - Core Evolutionary Engine and CAS (`rsicraft-core`)
- [x] L1 `Command::dispatch` universal evolutionary execution bus (26 commands).
- [x] 4-operator search space (`DRAFT`, `IMPROVE`, `DEBUG`, `CROSSOVER`).
- [x] Composite utility parent selector with roulette-wheel sampling.
- [x] Stigmergic SQLite WAL + BLAKE3/Zstd Content-Addressable Storage (CAS).
- [x] Multi-language sandboxed evaluator (Python 3.12, native Rust).
- [x] Zero-panic clippy lints (`unwrap`, `expect`, `panic`, `todo`, `unimplemented` denied). There is no `forbid(unsafe_code)` attribute.

### Tier 2 - Headless Driver (`rsicraft-mcp`)
- [x] Unified CLI binary `rsicraft` with `--json` output.
- [x] Hand-rolled MCP `2026-07-28` stdio server (26 tools). This is not a FastMCP server.
- [x] LLM endpoint reachability and latency probing.
- [x] Headless `--smoke` on the CLI (`rsicraft --smoke`, or `--db` plus `--smoke`). It checks status, hardware, help, and the 26-tool count. MCP stdio smoke stays the initialize probe in `scripts/release.ps1`.

### Tier 3 - Desktop Workstation UI (`rsicraft-app`)
- [x] Immediate-mode `egui` 0.37 desktop studio.
- [x] Interactive Bezier Lineage DAG canvas.
- [x] 4-operator launchpad, score spark charts, and unified diff inspector.
- [x] Seven sidebar pages. The View menu and the sidebar both switch them.

### Tier 4 - Craft Menu Bar Standard, Help Suite and Packaging
- [x] Universal 8-menu bar (`File`, `Edit`, `View`, `Insert`, `Modify`, `Commands`, `Window`, `Help`) with `switch_on_hover`.
- [x] 5-dialog Help suite (Command Palette, Keyboard Shortcuts, Project Files and Docs, Licenses and Notices, About RSICraft).
- [x] View menu switches the seven studio pages. Undo and redo restore store snapshots. Open Database and Save As use a file dialog (`rfd`). Cut, paste, and select all still have no selection.
- [x] Tofu Guard verification (`glyph_allowlist_no_tofu_chars`).
- [x] Desktop `--smoke` check.
- [x] Release packaging scripts and installer manifest (`scripts/release.ps1`, `installer/craft-installer.nsi`).
