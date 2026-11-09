# AGENTS.md - rsicraft

Sovereign Recursive Self-Improvement (RSI) Engine in Pure Rust (Frontis-MA1 30B/35B MoE + OpenRSI on Goliath RTX 4090).
Read first: `mcp-central-docs/standards/RUST_ARTCRAFT_STANDARD.md`.

## Layout

- `crates/rsicraft-store` (L1): Stigmergic SQLite WAL database (`tasks`, `experience_cards`, `task_frontiers`), BLAKE3 + Zstd Content-Addressable Storage (`cas.rs`), and composite utility parent selector (`utility.rs`).
- `crates/rsicraft-core` (L1): 4-operator evolutionary engine (`DRAFT`, `IMPROVE`, `DEBUG`, `CROSSOVER`), Frontis-MA1 `llama-server` client (`llm.rs`), isolated Docker + deterministic Demo sandbox runner (`sandbox.rs`), and Universal `Command::dispatch` bus.
- `crates/rsicraft-mcp` (L2): `rsicraft` binary combining CLI verbs (`--json`) and the MCP `2024-11-05` stdio server.
- `sandboxes/Dockerfile.evaluator`: Hardened non-root Python 3.11 evaluator container image.
- `schema.sql`: Canonical SQLite WAL schema DDL.

## Invariants (never weaken)

- Zero-panic policy: `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented` are denied across all crates outside tests.
- MCP stdio invariant: in `mcp` mode, `stdout` carries 100% pure line-delimited JSON-RPC; all diagnostics go to `stderr`.
- Composite utility formula: `U_i = lambda_s * s_norm_i + lambda_delta * delta_norm_i + lambda_nu * nu_i` with `nu_i = 1 / (1 + children_count_i)` and defaults `0.50 / 0.35 / 0.15`.
- Operator target temperatures: `DRAFT = 0.70`, `IMPROVE = 0.40`, `DEBUG = 0.20`, `CROSSOVER = 0.50`.
- Container sandbox isolation: `--network none`, `--memory 4g`, `--cpus 4`, `--read-only`, `--tmpfs /tmp:rw,noexec,nosuid,size=64m`.
- Demo data only via `demo-seed` on an empty database, explicitly labelled `DEMO`.

## Gates

1. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
2. `cargo test --workspace`
3. `cargo fmt --all --check`
4. MCP probe: `initialize` -> `tools/list` -> `tools/call`, stdout pure JSON
5. `Cargo.lock` committed, clean `git status`
6. `CHANGELOG.md` checkpoint line updated
