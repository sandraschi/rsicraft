# Benchmark Report: Amplification on pack-v2 (`amp-v2-qwen9b`)

**Run:** 2026-11-12 12:39 to 15:04, completed. 15 of 15 rows.
**Question:** on quality-scored tasks, does search beat the first working candidate by enough to justify training?
**Short answer:** no. The amplification gate (median 1.5 over at least 3 trials) was not met. Most runs never produced a working program, so the ratio is undefined.

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

`bench.log` records two repeating failures on the aborted runs: the completion budget was spent on reasoning and no code was drafted, and Ollama returned HTTP 500. That is a lead, not a finished diagnosis.

## What this does to the plan

pack-v2 does not qualify for training. The 35B pack-v2 comparison stays blocked until a pack produces a working candidate on most runs. The earlier pack-v1 result (median amplification 1.18) is unchanged: [BENCH_MODELSWAP_V2.md](BENCH_MODELSWAP_V2.md).
