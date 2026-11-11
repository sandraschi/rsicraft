# RSICraft — Tiered Execution Plan

**Parent Standard:** [`RUST_ARTCRAFT_STANDARD.md`](../../mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md)  
**Menu Bar Standard:** [`CRAFT_MENU_BAR_STANDARD.md`](../../mcp-central-docs/standards/rust-artcraft/CRAFT_MENU_BAR_STANDARD.md)  
**Architecture Model:** 4-Tier ArtCraft Incremental Delivery  
**Status:** Active  

---

## Architecture Flow

```
Tier 1: Core Engine ──> Tier 2: MCP / CLI ──> Tier 3: Desktop UI ──> Tier 4: Menu Bar, Help & Packaging
(rsicraft-core)         (rsicraft-mcp)         (rsicraft-app)         (Hardened Workstation)
```

---

## Tier Specifications

### Tier 1 — Core Evolutionary Engine & CAS (`rsicraft-core`)
- [x] L1 `Command::dispatch` universal evolutionary execution bus.
- [x] 4-operator search space (`DRAFT`, `IMPROVE`, `DEBUG`, `CROSSOVER`).
- [x] Composite utility parent selector with roulette-wheel sampling.
- [x] Stigmergic SQLite WAL + BLAKE3/Zstd Content-Addressable Storage (CAS).
- [x] Multi-language sandboxed evaluator (Python 3.12, native Rust).
- [x] `#![forbid(unsafe_code)]` and zero-panic clippy lints.

### Tier 2 — Headless Driver & FastMCP (`rsicraft-mcp`)
- [x] Unified CLI `rsicraft-cli` with `--json` output.
- [x] FastMCP stdio server communicating over JSON-RPC 2.0 (26 tools).
- [x] LLM endpoint reachability and latency probing.
- [x] Headless `--smoke` verification without window initialization.

### Tier 3 — Desktop Workstation UI (`rsicraft-app`)
- [x] Immediate-mode `egui` 0.37 / `wgpu` GPU rendering at 60–144 FPS.
- [x] Interactive Bézier Lineage DAG Canvas connecting candidates and tasks.
- [x] 4-operator launchpad, score spark charts, and unified diff inspector.

### Tier 4 — Craft Menu Bar Standard, Help Suite & Hardened Verification
- [x] Universal 8-menu bar (`File`, `Edit`, `View`, `Insert`, `Modify`, `Commands`, `Window`, `Help`) with `switch_on_hover`.
- [x] Mandatory 5-dialog Help suite (`Command Palette...`, `Keyboard Shortcuts...`, `Project Files & Docs...`, `Licenses & Notices...`, `About RSICraft`).
- [x] Fleet UI/UX Pattern 2 adherence: viewport-centric navigation, top button row mapped to View toggles, panel visibility managed via `View` / `Window`.
- [x] Tofu Guard verification (`glyph_allowlist_no_tofu_chars`).
- [x] Headless `--smoke` parity validation.
- [x] Release packaging and installer manifests.
