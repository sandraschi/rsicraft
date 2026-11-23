# Contributing to RSICraft

Contributions are welcome! Every pull request must satisfy the **6 Pure-Rust ArtCraft Merge Gates**:

1. **Zero Clippy Warnings & Zero Panics:**
   ```powershell
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   ```
   All crates enforce `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]`.
2. **All Tests Green (Including Tofu Glyph Guard):**
   ```powershell
   cargo test --workspace
   ```
3. **Formatting:**
   ```powershell
   cargo fmt --all --check
   ```
4. **100% CLI & MCP `2026-07-28` Parity:**
   Any new `Command` variant in `rsicraft-core` must be exposed in both `rsicraft_core::tools()` and `rsicraft-mcp`'s `Cmd` enum, and verified by `cli_verbs_map_to_known_tools`.
5. **Headless Desktop Studio Smoke Test:**
   ```powershell
   cargo run -p rsicraft-app -- --smoke
   ```
6. **Documentation & Brochure Sync:**
   Update `HELP.md`, `docs/TOOLS.md`, and `docs/brochure/rsicraft-brochure.{html,epub,pdf}` when changing tools or architecture.
