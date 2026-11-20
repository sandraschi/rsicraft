---
name: rsicraft-ergonomics
description: Search tuning, utility hyperparameter balancing, safe sandboxing, and store hygiene in RSICraft.
---

# RSICraft Search Ergonomics & Safety

Use this skill when tuning evolutionary search pressure, diagnosing plateaus, or keeping local sandboxes and SQLite stores healthy.

## 1. Tuning Composite Utility Weights

Parent selection balances three normalized components:

- `w_score`: Primary evaluator score.
- `w_delta`: Score gain over the parent card.
- `w_novelty`: Exploration bonus (`1 / (1 + child_count)`).

Workflow:

1. Call `rsi_hyperparams_get` to inspect current weights.
2. If search stagnates on a single local optimum, increase `w_novelty` with `rsi_hyperparams_set` and trigger `rsi_step` with `operator: "CROSSOVER"`.
3. If the frontier has many valid solutions but weak hill-climbing, raise `w_score` and `w_delta` with `rsi_hyperparams_set`.
4. Call `rsi_bandit_status` to review advisory per-operator success rates and Beta posterior recommendations.

## 2. Sandbox Safety Gates

- `sandbox_mode: "demo"`: Deterministic offline oracle; zero host execution.
- `sandbox_mode: "local"`: Runs Python or `rustc` directly as the current OS user. **Only use for trusted tasks and evaluators.**
- `sandbox_mode: "docker"`: Runs in an isolated container (`rsicraft-evaluator:latest`) with no network, read-only root, and memory/CPU limits.

Always test a new evaluator with `rsi_evaluate` (`sandbox_mode: "demo"` or `"docker"`) before running `rsi_campaign`.

## 3. Store Hygiene & Rung 2 Discipline

- Before pruning or deleting, export a backup with `rsi_export`.
- Use `rsi_cards_prune` (`keep_frontier: 10` or higher) to remove low-utility non-frontier cards after long campaigns.
- Only use `rsi_task_delete` when you intend to cascade-delete a task and all its cards permanently.
- Keep Rung 1 and Rung 2 distinct: `rsi_train_step` (`dry_run: true` first) only produces a candidate LoRA adapter; never treat a candidate as an improvement unless it passes `rsi_promote_gate` (Welch's t >= 2).
- Read any bundled skill on demand with `rsicraft_skill_read`.
