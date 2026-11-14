# Benchmark Report: Amplification on pack-v2 (`amp-v2-qwen9b`)

**Run:** 2026-11-12 12:39 to 15:04, completed. 15 of 15 rows.
**Question:** on quality-scored tasks, does search beat the first working candidate by enough to justify training?
**Short answer:** not measured. This run is invalid as evidence about search or the model.

**Correction (2026-11-14):** the harness had a defect during this run. When a reasoning model's final answer had no code block, the extractor took a code fence from its reasoning text. That was often a fragment echoed from the prompt (for example a fence opening followed by "markdown block"), which was then evaluated as if it were a program. 44% of the cards in this run had no `solve()` at all. The failures below therefore mostly measure the extractor, not the model. Fixed in commit 212f2d5: the prompt no longer contains a literal fence, only a closed fence that defines a function is taken from reasoning text, and an answer without a top-level function is recorded as `no_code` and not evaluated. The gate (median amplification 1.5 over at least 3 trials) remains open until pack-v2 is rerun with the fixed harness. The table is kept as the raw record of the defective run.

Raw data: `%LOCALAPPDATA%\rsicraft\bench\amp-v2-qwen9b\` (`bench.log`, `results.csv`, one campaign JSON per run).

## Setup

- Harness: `scripts/bench-models.ps1`, fixed DRAFT / IMPROVE / DEBUG / CROSSOVER cycle, 12 steps requested, local sandbox.
- Model: Ollama alias `rsi-qwen9b` (`pdurugyan/qwen3.5-9b-deepseek-v4-flash-Q4_K_M-v_2`, `num_ctx 16384`).
- Tasks: `tasks/pack-v2` (tsp, maxcut, regression as train; schedule, classify as held-out). Scores are solution quality.

## Result

| Task | Trial | Steps run | Skipped | Aborted | Success cards | Best score |
| :--- | ---: | ---: | ---: | :--- | ---: | ---: |
| classify | 1 | 10 | 2 | no | 0 | 0 |
| maxcut | 1 | 1 | 3 | yes | 0 | 0 |
| regression | 1 | 2 | 3 | yes | 0 | 0 |
| schedule | 1 | 0 | 0 | yes | 0 | 0 |
| tsp | 1 | 0 | 0 | yes | 0 | 0 |
| classify | 2 | 10 | 2 | yes | 0 | 0 |
| maxcut | 2 | 2 | 3 | yes | 0 | 0 |
| regression | 2 | 2 | 2 | yes | 0 | 0 |
| schedule | 2 | 2 | 3 | yes | 1 | 252.67 |
| tsp | 2 | 0 | 0 | yes | 0 | 0 |
| classify | 3 | 10 | 2 | no | 4 | 64.25 |
| maxcut | 3 | 4 | 5 | yes | 0 | 0 |
| regression | 3 | 8 | 4 | no | 0 | 0 |
| schedule | 3 | 0 | 0 | yes | 0 | 0 |
| tsp | 3 | 3 | 2 | yes | 0 | 0 |

13 of 15 runs have zero success cards. The two that scored are schedule trial 2 (one card, best 252.67, first success at step 2) and classify trial 3 (four cards, best 64.25, first success at step 3). A median amplification across tasks cannot be computed from this table.

The 15 campaign JSON files under `%LOCALAPPDATA%\rsicraft\bench\amp-v2-qwen9b\rsi-qwen9b_latest\` split into two failures and nothing else.

- 4 files are a bare error, with no campaign report: schedule trials 1 and 3 (completion budget spent on reasoning, no code), and tsp trials 1 and 2 (Ollama HTTP 500).
- The other 11 reports list 31 skipped steps: 23 completion-budget, 8 HTTP 500. Eight of those reports aborted. Three ran to the end (classify trial 1, best 0; classify trial 3, best 64.25; regression trial 3, best 0).

`DEFAULT_MAX_TOKENS` is 12,288. `ollama show rsi-qwen9b` has `num_ctx 16384`. On 2026-11-12 Ollama started that alias as llama-server `-c 32768 -np 2`, so each slot is 16,384. The cap was in effect. Raising the completion budget still shrinks the room left for the prompt, and the default stays 12,288.

The HTTP 500s are not a prompt rejected up front for being too long. `%LOCALAPPDATA%\Ollama\server.log` shows the request already generating, then `POST /v1/chat/completions` returns 500 and llama-server reloads:

- 14:03:03 (tsp trial 2 in `bench.log`): 1m50s, `n_gen` 12,202, then a reload of the same 9B weights.
- 13:18:03 (tsp trial 1): `n_gen` already past 11,000. At the same second Ollama predicted 36.6 GiB for a different model at `num_ctx 262144`, evicted the 9B runner, and the other client's `POST /api/generate` ended 499.
- 12:48:03 is the short one: 2.16s, while the prompt cache already held a 16,383-token entry in a 16,384 slot.

A 500 at the token cap and a 200 with an empty answer are both the completion budget. The 262,144-context load is a second client on the same Ollama. The next non-2xx body is kept on the step error (`format_status_failure`), so a later run does not have to be read out of `server.log`.

## What this does to the plan

pack-v2 does not qualify for training. The 35B pack-v2 comparison stays blocked until a pack produces a working candidate on most runs. The earlier pack-v1 result (median amplification 1.18) is unchanged: [BENCH_MODELSWAP_V2.md](BENCH_MODELSWAP_V2.md).
