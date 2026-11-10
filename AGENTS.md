# RSICraft - Per-Repo Agent Instructions (`AGENTS.md`)

**Purpose:** Navigation map and hard invariants for agents working in `D:\Dev\repos\rsicraft`.

## Fleet Reading Chain

1. [`../AGENTS.md`](../AGENTS.md) - Fleet root instructions and `AGENT_READING_ORDER.md` pointer.
2. [`../mcp-central-docs/standards/rust-artcraft/RUST_ARTCRAFT_STANDARD.md`](../mcp-central-docs/standards/rust-artcraft/RUST_ARTCRAFT_STANDARD.md) - Pure-Rust ArtCraft 3-Crate Standard & 6 Merge Gates.
3. [`../mcp-central-docs/standards/CRAFT_BROCHURE_STANDARD.md`](../mcp-central-docs/standards/CRAFT_BROCHURE_STANDARD.md) - Technical Brochure (`HTML` + `EPUB` + `PDF`) specification.

## Workspace Layout (3-Crate Pure-Rust ArtCraft Standard)

| Path | Role |
| :--- | :--- |
| `crates/rsicraft-core/` | `L1 Core + Store`: 4-operator action space and language-aware prompts (`operators.rs`), `LlamaClient` (`llm.rs`), `SandboxRunner` (`sandbox.rs`), 9-tab help (`help.rs`), 22-tool `Command::dispatch` bus (`lib.rs`), and the store module (`store/mod.rs` SQLite `WAL` registry, `store/cas.rs` BLAKE3 + Zstd CAS, `store/utility.rs` composite utility). |
| `crates/rsicraft-mcp/` | `L2 CLI + MCP`: `clap` CLI (`--json` + table/help renderer) and pure-Rust MCP `2024-11-05` stdio loop (`mcp.rs`). |
| `crates/rsicraft-app/` | `L3 Desktop Studio`: `eframe`/`egui 0.37` app calling `rsicraft_core::dispatch` in-process (single `main.rs`). |
| `schema.sql` | Store schema, embedded into `rsicraft-core` with `include_str!`. Additive changes only (`CREATE TABLE IF NOT EXISTS`); it runs on every open. |
| `sandboxes/Dockerfile.evaluator` | Non-root Python + NumPy/SciPy evaluator image for `sandbox_mode = "docker"`. |
| `docs/brochure/` | `rsicraft-brochure.{html,epub,pdf}` compiled via `BookCraft`. |
| `docs/help/index.html` | Interactive 9-tab HTML Help Studio. |

## Hard Invariants

1. **Zero Panics:** Every crate MUST keep `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]`.
2. **100% CLI / MCP Parity:** Every MCP tool in `rsicraft_core::tools()` must map through `Command::from_tool` and have a matching CLI verb tested in `cli_verbs_map_to_known_tools`. A value the dispatcher accepts must also be listed in the tool's JSON schema `enum`.
3. **Store mutations are transactional:** Any `Registry` method that runs more than one write statement goes through `with_tx`. Never toggle `PRAGMA foreign_keys`; use `PRAGMA defer_foreign_keys = ON` inside the transaction when insert order is unknown.
4. **Sandbox children go through `run_captured`:** No `.output()` / piped `wait` on a child that runs candidate code. Output is captured to files and the host-side reaper enforces the deadline. Metrics are parsed from the evaluator's stdout only.
5. **Docs must match the code on safety claims:** The Docker flags listed in `docs/ARCHITECTURE.md` and `docs/CONFIGURATION.md` are the ones in `run_docker_evaluation`. Change both together.
6. **Brochure Sync:** Whenever tools or architecture change, update `docs/brochure/rsicraft-brochure.html` and recompile `.epub` and `.pdf` via:
   ```powershell
   pwsh -File D:\Dev\repos\mcp-central-docs\scripts\build-craft-brochure.ps1 -RepoPath D:\Dev\repos\rsicraft -PreserveExistingHtml
   ```
7. **All 6 Gates Before Commit:** Run `just ci` (`cargo clippy -- -D warnings`, `cargo test --workspace`, `cargo fmt --all --check`).

## Repo Hygiene

1. **LF only.** Every text file is LF (`.gitattributes` enforces `eol=lf`; `*.bat` is the one CRLF exception). Tools that write CRLF on Windows must be followed by a normalization pass before commit.
2. **No em dash (U+2014), anywhere, any filetype.** Use a plain hyphen or restructure the sentence.
3. **Single trunk.** Work on `main`. No long-lived agent branches, no leftover WIP branches or stashes; if a branch is unavoidable, merge or delete it in the same session.
4. **Releases ship executables directly.** `rsicraft.exe` and `rsicraft-app.exe` are release assets as plain files. A zip is at most an extra beside them, never the only download.
