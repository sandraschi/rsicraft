# Benchmark Report: Model Swap on pack-v1 (`modelswap-v2`)

**Run:** 2026-11-11 16:57 to 2026-11-12 12:33, stopped by hand after 26 of 36 runs.
**Question:** with the harness fixed, does Frontis-MA1-35B beat its base model Qwen3.6-35B-A3B on rsicraft's own tasks, the way the paper reports for MLE-Bench Lite?
**Short answer:** not on these tasks. And the more important finding is about the tasks, not the models.

Raw data: `%LOCALAPPDATA%\rsicraft\bench\modelswap-v2\` (`results.csv`, one SQLite database per run). Re-create the tables with `scripts\bench-analyze.ps1 -Dir <that directory>`.

## Setup

- Harness: rsicraft `c9eee3c`, fixed DRAFT / IMPROVE / DEBUG / CROSSOVER cycle, 12 steps per run, local sandbox, 60 s sandbox timeout.
- Models (Ollama, Q4_K_M, `num_ctx 16384`): `rsi-base-35b` (Qwen3.6-35B-A3B) and `rsi-frontis-35b` (Frontis-MA1-35B).
- Tasks: `tasks/pack-v1`, six CPU-only algorithm tasks scored as speedup over a naive reference timed in the same process.
- Completed: base model, all 3 trials (18 runs). Frontis-MA1, trial 1 (6 runs) plus 2 runs of trial 2, one of which failed on an Ollama HTTP 500.

## Result 1: no Frontis-MA1 advantage on pack-v1

Best score per run (speedup over the naive reference):

| Task | Split | Base t1 | Base t2 | Base t3 | Frontis t1 | Frontis t2 |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: |
| primes | train | 1400.36 | 1413.90 | 1071.18 | 367.36 | |
| lis | train | 426.18 | 317.12 | 411.63 | 551.31 | |
| inversions | heldout | 55.75 | 33.50 | aborted | 40.06 | 46.46 |
| wordfreq | heldout | 26.65 | 20.44 | 20.11 | 22.72 | |
| edit-distance | train | 2.83 | 3.90 | 2.30 | 2.04 | failed |
| matmul | train | 3.00 | 3.05 | 2.58 | 2.55 | |

Frontis-MA1 is inside or below the base model's range on five of six tasks and above it on one (lis).

What this does and does not show:
- It is one complete Frontis trial. It cannot support "Frontis is worse".
- The tasks are outside Frontis-MA1's training domain (machine-learning engineering). The paper's claim is about that domain and is not contradicted here.
- It does show that the paper's model-swap gain does not transfer to generic algorithm speed tasks at a 12-step budget.

## Result 2: search adds little on pack-v1

For each run: the first working candidate, the final best, and their ratio ("amplification").

| Task | Median amplification | Range | Runs whose best card is a fresh DRAFT |
| :--- | ---: | :--- | :--- |
| primes | 4.65 | 3.02 to 8.76 | 0 of 4 |
| lis | 1.20 | 1.00 to 2.06 | 3 of 5 |
| wordfreq | 1.26 | 1.11 to 1.48 | 2 of 4 |
| inversions | 1.13 | 1.00 to 1.32 | 1 of 4 |
| matmul | 1.12 | 1.00 to 1.29 | 2 of 4 |
| edit-distance | 1.11 | 1.00 to 1.39 | 1 of 4 |

- Median over all 25 runs with a working candidate: **1.18**.
- Identical setups differ by 20 to 30% between trials (base model: primes 1071 to 1414, lis 317 to 426, wordfreq 20 to 27). An amplification of 1.18 is inside that noise.
- In 9 of 25 runs the best card was a DRAFT. That is resampling the model, not searching.
- Only primes shows real amplification.

**Why this matters more than Result 1:** a self-improvement loop distills what search found back into the model. Where search finds nothing beyond the model's first working answer, there is nothing to distill. A 35B model already knows the textbook solution to five of these six tasks, so pack-v1 cannot show the effect regardless of how good the training code is.

## Result 3: operational problems

- **Skipped steps:** `results.csv` has 81 skipped steps and 178 executed steps (31% of attempted steps). The per-step error text was not saved (the script now writes `<task>-t<n>.json`; this run predates that). The same Ollama log, through the night of 2026-11-12, shows `POST /v1/chat/completions` returning 500 while a slot is still generating, then a runner reload. One of those is 03:18:05, 5m00s, `n_gen` 5,647, which is the base inversions trial 3 failure in `bench.log`. There is no per-step split of budget versus 500 for this run.
- **Aborted runs:** 9 of 26 runs ended early. 2 produced no card at all, both HTTP 500: base inversions trial 3 and Frontis edit-distance trial 2.
- **Speed:** about 50 minutes per 12-step run on the 35B models, so 4 to 5 minutes per step. A full 36-run comparison is about 30 hours.
- **Alias mismatch, still present on 2026-11-14:** `ollama show` was not changed. `rsi-frontis-35b` is `TEMPLATE {{ .Prompt }}` and `PARAMETER num_ctx 16384`. `rsi-base-35b` adds `temperature 0.3`, stops on `<|im_start|>`, `<|im_end|>`, `<think>`, and the literal `.Prompt }}<|im_end|>`, and its template prints ` .Prompt }}` where the prompt action should be. The two aliases are not parameter-identical. They were not rewritten in this pass.

## Decisions taken

1. Stop the run. The remaining ten Frontis runs would not change either result.
2. **Amplification becomes a gate.** A task pack qualifies for training only if the final best clearly beats the first working candidate, beyond trial-to-trial noise.
3. New task pack `tasks/pack-v2`: problems without a textbook optimum (TSP heuristic, max-cut, weighted-tardiness scheduling) and small machine-learning tasks without libraries (regression, classification). Scores are solution quality, not timing, so they do not carry timing noise. The two ML tasks are in Frontis-MA1's home domain.
4. The loop experiment moves to a 9B-class model: faster, more headroom, and trainable on a 24 GB card.
5. Diagnose and fix the skip rate before spending more 35B hours.
