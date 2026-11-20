---
name: rsicraft-workflow
description: End-to-end evolutionary program search workflow in RSICraft using MCP tools (probe, task setup, campaign, frontier inspection, diffing, and dataset export).
---

# RSICraft Evolutionary Search Workflow

Use this skill when driving a program-search campaign through `rsicraft` MCP tools.

## 1. Preflight & Endpoint Check

1. Call `rsi_status` to inspect the SQLite store (task count, card count, active DB path).
2. If running against a real local LLM (`provider: "llama"`), call `rsi_llm_probe` first to verify the server is reachable, the requested model is served, and context-window settings are sane.
3. For an offline dry run or UI check, call `rsi_demo_seed` to populate the `opt-synth` benchmark task.

## 2. Register or Inspect a Task

- List existing tasks with `rsi_task_list` or inspect one with `rsi_task_show`.
- Create or update a task with `rsi_task_upsert` (`task_id`, `name`, `description`, `evaluator_script`).
- Ensure `evaluator_script` prints a single `RSI_METRIC_JSON={"status":"SUCCESS","score":<number>}` marker on stdout.
- Validate a baseline candidate before starting a search using `rsi_evaluate` with `record: false` (or `record: true` to seed the frontier).

## 3. Run the Evolutionary Loop

- Launch a multi-step campaign with `rsi_campaign` (`task_id`, `steps`, `provider`, `sandbox_mode`, `timeout_secs`).
  - Use `sandbox_mode: "docker"` for untrusted model-generated code; use `"local"` only for trusted code.
- Or drive individual operators with `rsi_step` (`operator`: `DRAFT`, `IMPROVE`, `DEBUG`, or `CROSSOVER`).

## 4. Inspect Frontier, Lineage & Diffs

- Call `rsi_get_frontier` to retrieve the top-K highest-utility cards for `task_id`.
- Call `rsi_card_list` (filterable by `operator` or `status`) and `rsi_card_show` to inspect decompressed source code, BLAKE3 hash, and sandbox logs.
- Call `rsi_lineage` for the full parent-child DAG, and `rsi_diff` (`card_a_id`, `card_b_id`) to inspect exact mutations between parent and child.

## 5. Export Lineages & Rung 2 Datasets

- Export a portable task bundle with `rsi_export` (and restore with `rsi_import`).
- Export curated SFT trajectories and DPO preference pairs with `rsi_dataset_export` (`format`: `"all"`, `"sft"`, or `"dpo"`; `min_delta`).
- If troubleshooting any `[llm:<class>]` error, call `rsi_help` with `tab: "troubleshoot"`.
