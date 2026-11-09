# Development, Testing & Brochure Workflow: RSICraft

**Navigation:** [README](../README.md) · [Architecture](ARCHITECTURE.md) · [Troubleshooting](TROUBLESHOOTING.md)

---

## 1. Running the 6 ArtCraft Quality Gates

```powershell
# 1. Lint gate (denies unwrap, expect, panic, todo, unimplemented, and warnings)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# 2. Unit & integration test suite (store, core, mcp protocol, app smoke & glyph guard)
cargo test --workspace

# 3. Formatting gate
cargo fmt --all --check

# 4. Headless Desktop Studio smoke verification
cargo run -p rsicraft-app -- --smoke

# 5. Launch the GPU Desktop Studio
cargo run -p rsicraft-app
```

---

## 2. Regenerating the Technical Brochure (`HTML` -> `EPUB` + `PDF`)

Whenever tool definitions, safety invariants, or architectural diagrams change:

1. Edit `docs/brochure/rsicraft-brochure.html`.
2. Compile the EPUB 3.0 and A4 PDF brochures via `BookCraft`:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File "D:\Dev\repos\mcp-central-docs\scripts\build-craft-brochure.ps1" `
  -RepoPath "D:\Dev\repos\rsicraft" -PreserveExistingHtml
```

3. Verify all three files exist and are non-empty in `docs/brochure/`:
   - `rsicraft-brochure.html`
   - `rsicraft-brochure.epub`
   - `rsicraft-brochure.pdf`
