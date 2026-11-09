# RSICraft — Per-Repo Agent Instructions (`AGENTS.md`)

**Purpose:** Navigation map and hard invariants for agents working in `D:\Dev\repos\rsicraft`.

## Fleet Reading Chain

1. [`../AGENTS.md`](../AGENTS.md) — Fleet root instructions and `AGENT_READING_ORDER.md` pointer.
2. [`../mcp-central-docs/standards/rust-artcraft/RUST_ARTCRAFT_STANDARD.md`](../mcp-central-docs/standards/rust-artcraft/RUST_ARTCRAFT_STANDARD.md) — Pure-Rust ArtCraft 3-Crate Standard & 6 Merge Gates.
3. [`../mcp-central-docs/standards/CRAFT_BROCHURE_STANDARD.md`](../mcp-central-docs/standards/CRAFT_BROCHURE_STANDARD.md) — Technical Brochure (`HTML` + `EPUB` + `PDF`) specification.

## Workspace Layout (3-Crate Pure-Rust ArtCraft Standard)

| Path | Role |
| :--- | :--- |
| `crates/rsicraft-store/` | `L1 Store`: SQLite `WAL` schema (`schema.sql`), BLAKE3 + Zstd CAS (`cas.rs`), composite utility parent selector (`utility.rs`). |
| `crates/rsicraft-core/` | `L1 Core`: 4-operator action space (`operators.rs`), `LlamaClient` (`llm.rs`), `SandboxRunner` (`sandbox.rs`), 7-tab help (`help.rs`), and 14-tool `Command::dispatch` bus (`lib.rs`). |
| `crates/rsicraft-mcp/` | `L2 CLI + MCP`: `clap` CLI (`--json` + table/help renderer) and pure-Rust MCP `2024-11-05` stdio loop (`mcp.rs`). |
| `sandboxes/Dockerfile.evaluator` | Rootless air-gapped Python 3.12 + NumPy/SciPy evaluator image. |
| `docs/brochure/` | `rsicraft-brochure.{html,epub,pdf}` compiled via `BookCraft`. |
| `docs/help/index.html` | Interactive 7-tab HTML Help Studio. |

## Hard Invariants

1. **Zero Panics:** Every crate MUST keep `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]`.
2. **100% CLI / MCP Parity:** Every MCP tool in `rsicraft_core::tools()` must map through `Command::from_tool` and have a matching CLI verb tested in `cli_verbs_map_to_known_tools`.
3. **Brochure Sync:** Whenever tools or architecture change, update `docs/brochure/rsicraft-brochure.html` and recompile `.epub` and `.pdf` via:
   ```powershell
   pwsh -File D:\Dev\repos\mcp-central-docs\scripts\build-craft-brochure.ps1 -RepoPath D:\Dev\repos\rsicraft -PreserveExistingHtml
   ```
4. **All 6 Gates Before Commit:** Run `just ci` (`cargo clippy -- -D warnings`, `cargo test --workspace`, `cargo fmt --all --check`).
