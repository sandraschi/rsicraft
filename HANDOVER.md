# RSICraft - Handover State

**Date:** 2026-11-12 (midday)
**Status:** Evolution loop real. First benchmark done: pack-v1 fails the amplification gate, no Frontis-MA1 advantage on it. Meta-evolution NOT reached, nothing trained.
**Read first:** `docs/FRONTIS_PARITY_PLAN.md` (state table, section 1a) and `docs/BENCH_MODELSWAP_V2.md`.

## What changed since 2026-11-10

- Model-swap benchmark `modelswap-v2` ran 26 of 36 runs and was stopped by hand. Findings:
  - Frontis-MA1-35B is inside or below the base model's range on five of six pack-v1 tasks (one complete trial).
  - Median search amplification (final best / first working candidate) is 1.18, inside the 20 to 30% trial noise. 9 of 25 runs had a fresh DRAFT as their best card.
  - 25 to 33% of 35B steps produced no card. Cause not diagnosed.
- New: `tasks/pack-v2` (tsp, maxcut, regression, schedule, classify; quality-scored, no timing noise), `scripts/bench-analyze.ps1`, and `bench-models.ps1` now saves the full campaign report per run.
- PRD rewritten around gated goals (PRD section 5). The amplification gate is now mandatory before any training.

## Running right now

Amplification measurement on pack-v2 with a 9B model, started 2026-11-12 12:39:

- Output: `%LOCALAPPDATA%\rsicraft\bench\amp-v2-qwen9b\` (`bench.log`, `results.csv`, one `.db` and one `.json` per run).
- Model alias `rsi-qwen9b` = `pdurugyan/qwen3.5-9b-deepseek-v4-flash-Q4_K_M-v_2` with `num_ctx 16384`. Without that cap Ollama loads this model with a 262,144-token context, 24 GB, spilling to CPU at 20 tokens/s.
- 5 tasks x 3 trials x 12 steps. It is resumable: rerun the same command and finished rows are skipped.
- When it ends: `powershell -File scripts\bench-analyze.ps1 -Dir $env:LOCALAPPDATA\rsicraft\bench\amp-v2-qwen9b`. The number to read is the median amplification; the gate is 1.5.

## Next steps, in order

1. Read the pack-v2 amplification result. If it passes, rerun Frontis-MA1 vs base on pack-v2. If it fails, design harder tasks or accept "search tool" as the scope.
2. Diagnose cardless steps on the 35B models from the saved `.json` campaign reports. Suspect: 12,288-token completion budget against a 16,384-token context.
3. Make `rsi-base-35b` and `rsi-frontis-35b` identical in parameters (the base alias has extra stop tokens and a temperature line).
4. Then the rest of Phase 1 in the plan (token accounting, `runs` table, real bandit, ablations).

## What is real, what is not

Unchanged from the state table in the plan, section 2. Short form:
- Real: evolution loop, sandboxes, store, three-phase step pipeline, worker-thread studio, real promotion gate, task packs, benchmark scripts.
- Real code, never run end to end: `rsi_train_step` with `sandboxes/train_lora.py`.
- Missing: adapter serving, held-out split in the store, token accounting, agentic steps.
- Mislabeled: `bandit.rs` is advisory statistics, not Thompson sampling, and campaigns do not use it.

## Verification

- Last full gate run (2026-11-10): clippy clean, fmt clean, 33 tests. No Rust source changed in this session.
- pack-v2 evaluators checked with a weak and a medium candidate each; all five grade as intended.
- Not verified: the egui Live Run panel on screen; `train_lora.py`; the promotion gate against two real models.

## Documentation debt

Release v0.3.0 is titled "4-rung Frontis ladder", and the brochure, `docs/help/index.html`, several `docs/*.md` essays and parts of the README still describe the full ladder as implemented. They are wrong. The README carries a status note at the top; the PRD, plan, handover and benchmark report are correct.

## Local environment notes (Goliath)

- Port 8081 (default `--llama-url`) is held by Docker. Use `--llama-url http://127.0.0.1:11434` for Ollama.
- Ollama aliases created for this project: `rsi-base-35b`, `rsi-frontis-35b`, `rsi-qwen9b`.
- Frontis-MA1 weights are CC BY-NC 4.0: personal and research use only.
