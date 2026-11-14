# RSICraft - Handover State

**Date:** 2026-11-14
**Status:** Tiers 1-4 built (core, 26-tool CLI/MCP with `--smoke`, studio, 8-menu bar, 10-tab manual, undo/redo of store snapshots, Open Database and Save As file dialogs). Search measured. Export, operator statistics, LoRA harness, and promotion gate are in the binary. LLM failures are classified. A campaign preflights the provider. An answer with no top-level function is `no_code`. pack-v1 and pack-v2 both fail the amplification gate. No generation trained. Cut, paste, and select all still have no selection.
**Read first:** `docs/FRONTIS_PARITY_PLAN.md` (state table, section 1a), `docs/BENCH_MODELSWAP_V2.md`, and `docs/BENCH_AMP_V2.md`.

## What changed since 2026-11-10

- Model-swap benchmark `modelswap-v2` ran 26 of 36 runs and was stopped by hand. Findings:
  - Frontis-MA1-35B is inside or below the base model's range on five of six pack-v1 tasks (one complete trial).
  - Median search amplification (final best / first working candidate) is 1.18, inside the 20 to 30% trial noise. 9 of 25 runs had a fresh DRAFT as their best card.
  - 81 of 259 attempted pack-v1 steps were skipped (31%). Two campaigns died on HTTP 500. Counted in `docs/BENCH_MODELSWAP_V2.md`.
- New: `tasks/pack-v2` (tsp, maxcut, regression, schedule, classify; quality-scored, no timing noise), `scripts/bench-analyze.ps1`, and `bench-models.ps1` now saves the full campaign report per run.
- PRD rewritten around gated goals (PRD section 5). The amplification gate is now mandatory before any training.

## pack-v2 result (finished 2026-11-12 15:04)

The 9B amplification run completed. Report: `docs/BENCH_AMP_V2.md`. Raw files: `%LOCALAPPDATA%\rsicraft\bench\amp-v2-qwen9b\`.

- 15 of 15 runs finished. 13 have zero success cards.
- The only scores are schedule trial 2 (252.67, one card) and classify trial 3 (64.25, four cards).
- The 1.5 amplification gate was not met. Do not train on this pack.
- The recorded failures are the completion budget and Ollama HTTP 500. Counted in `docs/BENCH_AMP_V2.md`. The current binary labels an answer with no top-level function as `no_code` and does not evaluate it.
- Model alias `rsi-qwen9b` needs `num_ctx 16384`. Without that cap Ollama loads a 262,144-token context and spills the 4090 to CPU.

## Next steps, in order

1. Treat the product as a search tool until a pack produces a working candidate on most runs. Designing a new pack is optional. Training is not next.
2. Pack-v2 9B is counted in `docs/BENCH_AMP_V2.md`. The HTTP 500s are the runner dying at the completion budget, or a second client loading a 262,144 context, not a prompt rejected up front. Pack-v1 has no per-step JSON: 81 of 259 attempted steps were skipped (31%), and 2 campaigns died on HTTP 500. The 12,288 default stays.
3. Done on 2026-11-14: `rsi-base-35b` was rewritten with `ollama create`. Both 35B aliases are now `TEMPLATE {{ .Prompt }}` and `PARAMETER num_ctx 16384`. The base weight blobs are unchanged. `scripts/align-35b-alias.ps1 -Apply` calls `ollama create -f` because `/api/create` no longer accepts a Modelfile string.
4. The 35B pack-v2 comparison stays blocked until step 2 is fixed.
5. Then the rest of Phase 1 in the plan (token accounting, `runs` table, real bandit, ablations).

## What is real, what is not

Unchanged from the state table in the plan, section 2. Short form:
- Real: evolution loop, sandboxes, store, three-phase step pipeline, worker-thread studio, real promotion gate, task packs, benchmark scripts.
- Real code, never run end to end: `rsi_train_step` with `sandboxes/train_lora.py`.
- Missing: adapter serving, held-out split in the store, token accounting, agentic steps.
- Mislabeled: `bandit.rs` is advisory statistics, not Thompson sampling, and campaigns do not use it.

## Verification

- 2026-11-14: `scripts/release.ps1` exited 0. Gates passed, release smoke passed (app, CLI `--smoke`, MCP initialize). Artifacts are in `dist\`: `rsicraft-app.exe` (14.2 MB), `rsicraft.exe` (4.9 MB), `rsicraft-0.3.1-setup.exe`, the portable zip, and `rsicraft-v0.3.1.mcpb`. Not published. Tag `v0.3.1` already exists, so a later publish needs a version bump. `CHANGELOG_LATEST.md` is still missing.
- pack-v2 evaluators were checked earlier with a weak and a medium candidate each.
- An uncommitted light-theme contrast edit in `crates/rsicraft-app/src/main.rs` was already in the tree. This pass did not make it.
- Not verified: the egui Live Run panel on screen; `train_lora.py`; the promotion gate against two real models.

## Documentation debt

The README, PRD, architecture page, tool list, and configuration page describe the studio that is in the binary: eight menus, seven pages, ten help tabs, classified LLM errors. `docs/GUIDE.md` and `docs/help/index.html` are generated from `help.rs` by `scripts/build-help-docs.ps1`. The changelog title of v0.3.0 still says "4-rung Frontis ladder"; the correction paragraph under it is the one to trust.

## Local environment notes (Goliath)

- Port 8081 (default `--llama-url`) is held by Docker. Use `--llama-url http://127.0.0.1:11434` for Ollama.
- Ollama aliases created for this project: `rsi-base-35b`, `rsi-frontis-35b`, `rsi-qwen9b`.
- Frontis-MA1 weights are CC BY-NC 4.0: personal and research use only.
