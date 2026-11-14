# Frontis Ladder Parity Plan

**Date:** 2026-11-10, revised 2026-11-12 after the first benchmark | **Status:** Phase 0 done, Phase 1 re-scoped (see section 1a) | **Source of truth for "what is real":** the table in section 2.

Reference: Frontis-MA1 / OpenMLE (`arXiv:2607.28568`, `FrontisAI/OpenRSI`).

---

## 1. What the paper claims, and what it does not

The paper's ladder as written has three rungs it defines itself:

| Rung | Definition | Who is there |
| :--- | :--- | :--- |
| Evolution | A model repeatedly modifies a candidate program using execution feedback. | AlphaEvolve, AIDE, OpenMLE-Evo, rsicraft |
| Meta-evolution | The trajectories of that search are used to train the model that proposes the next modifications. | Frontis-MA1 (one generation) |
| RSI | A sustained loop in which each upgraded system improves the process that builds its successor. | Nobody, including the paper |

Measured results worth copying the method for:

- Same harness, model swapped (Qwen3.6-35B-A3B to Frontis-MA1-35B): 39.39% to 60.61% medal average on MLE-Bench Lite (22 tasks, 12 h per task, 3 runs).
- Same model, harness swapped (AIRA-Evo to OpenMLE-Evo): 53.03% to 60.61%, with 41.7% fewer tokens.
- Held-out NatureBench Lite (10 tasks): match-SOTA 50% to 70% from the model swap alone.

What the paper says it has not done: the operators are trained once ("generation 1"), no second generation trained on its own search traces is reported, the evolutionary system itself does not evolve, and parent selection is a fixed three-factor formula the authors call a preliminary prototype.

So "full parity with the ladder" has an exact meaning: reach **meta-evolution with one measured generation**. Anything past that is new research, not parity.

**License constraint:** the OpenRSI code is CC BY-NC 4.0. rsicraft is MIT/Apache. Their code cannot be copied into this repo. The ideas in the paper can be reimplemented, and their released datasets and weights can be used under their own terms (check the model cards before redistributing anything derived).

---

## 1a. First measurements and what they changed (2026-11-12)

Full report: [BENCH_MODELSWAP_V2.md](BENCH_MODELSWAP_V2.md).

- **Model swap on pack-v1:** Frontis-MA1-35B showed no advantage over its base model on rsicraft's six algorithm tasks (one complete trial; inside or below the base range on five of six tasks). The tasks are outside its training domain, so this does not contradict the paper, and it does not reproduce it.
- **Search amplification on pack-v1 is 1.18 (median):** the final best is barely better than the first working candidate, inside the 20 to 30% trial-to-trial noise. In 9 of 25 runs the best card was a fresh DRAFT. Only one task (primes) showed real amplification.
- **Consequence:** a loop that distills search results into the model has nothing to distill on tasks like these. Before any training, a task pack must pass an **amplification gate**.

Current estimates of success (judgment, not measurement):

| Goal | Estimate |
| :--- | :--- |
| A solid, honest evolutionary-search craft app | about 90% |
| Reproduce the paper's model-swap gain on pack-v1 | about 15% |
| One generation G1 clearly better than G0 on held-out tasks, trained only on its own traces | about 25%, rising to 40 to 50% if pack-v2 passes the amplification gate |
| The same with a small model trained on a larger model's traces (distillation, must be labelled as such) | about 60% |
| Sustained recursion (G2 better than G1 better than G0) | under 5% |

The most achievable real result is narrower than "RSI": a fine-tuned small model that wastes fewer steps (higher share of valid candidates, fewer steps to a threshold).

---

## 2. Honest state of rsicraft

| Capability | State | Evidence |
| :--- | :--- | :--- |
| Evolution loop (4 operators, CAS store, lineage, frontier) | **Real** | 33 tests; live run against Ollama `gemma4:12b` with the local sandbox |
| Docker and local sandboxes, host-side reaper | **Real** | exercised live 2026-11-10 |
| Multi-trial evaluation (`trials`) | **Real in the sandbox, unused by steps** | steps call `evaluate(..., 1)` |
| SFT/DPO trace export | **Real but naive** | prompts are re-synthesized, not the prompts the model actually saw; hardcoded `>= 70.0` score threshold |
| Operator bandit (`rsi_bandit_status`) | **Advisory only, mislabeled** | not Thompson sampling (posterior mean plus hand-set biases) and not wired into campaigns |
| Model generations on a task | **Real since today** | steps now send the task's promoted alias to the endpoint; before today the generation number was cosmetic |
| LoRA training (`rsi_train_step`) | **Real code path, never run end to end** | `train_lora.py` is a standard PEFT script, unverified on Goliath |
| Task packs and model-swap benchmark | **Real** | `tasks/pack-v1`, `tasks/pack-v2`, `scripts/bench-models.ps1`, `scripts/bench-analyze.ps1`. pack-v1 report: `BENCH_MODELSWAP_V2.md`. pack-v2 9B report: `BENCH_AMP_V2.md` (gate failed) |
| Promotion gate (`rsi_promote_gate`) | **Real** | A/B of two served aliases over fresh searches, Welch t >= 2; refuses the demo sandbox |
| Serving a trained adapter | **Missing** | no adapter-to-GGUF conversion, no endpoint registration |
| Held-out task split | **In the pack files only** | each task file declares `split`; the store and the gate do not use it yet |
| Cross-task training data | **Missing** | one task yields tens of traces; the paper used 26,259 |
| Agentic operators (tool use inside a step) | **Missing** | one completion per step |
| Desktop app runs off the UI thread | **Real since today** | worker thread, stop button, live timeline |

What was removed today: a training step that printed a hardcoded loss curve and advanced the generation without training, and a promotion gate that awarded every candidate a constant +4.5% and promoted it. Both were reported as "complete, no blockers".

---

## 3. Blockers on a single RTX 4090

1. **VRAM is one pool.** A 35B model at Q4 is about 21.5 GB. That leaves nothing for a GPU task in the sandbox and nothing for a resident model. The paper's "12 GB cap" is the task budget; the model was not sharing that card. *Way through:* CPU-only tasks first (algorithms, tabular ML), or a 9B-class model for anything that must share the card.
2. **Reasoning models burn the step budget.** Measured today: `gemma4:12b` spends 5,000 to 6,000 tokens thinking per step, 30 to 70 s per step, and sometimes exhausts the budget with no answer. *Done:* 12,288-token default, salvage of the last drafted code block, dedent of nested fences. *Still open:* per-step token accounting.
3. **Fine-tuning the 35B model locally is not realistic.** 4-bit weights alone fill the card before activations and gradients. *Way through:* run the loop on a model that fits (Qwen 9B class, QLoRA in WSL Ubuntu), which is also faster to iterate on. The claim being tested is the loop, not the model size.
4. **Data volume.** A generation needs thousands of traces across many tasks. *Way through:* a task pack (section 4, Phase 1) plus overnight campaigns; optionally bootstrap from the released OpenMLE SFT traces.
5. **Serving the adapter.** llama.cpp loads LoRA adapters converted to GGUF (`convert_lora_to_gguf.py`, `llama-server --lora`); Ollama takes an `ADAPTER` line in a Modelfile. Neither is wired up or tested here.
6. **Statistics.** One timing is noise. A gate at 5 trials x 6 steps x 2 models is 60 steps, about an hour on the 12B model. Budget for it; do not shortcut it.
7. **Port and model hygiene on Goliath.** Port 8081 (the default `llama_url`) is held by Docker. Loading the 35B model evicts whatever is resident.

None of these is a wall. 1 and 3 mean the honest local experiment uses a small model and CPU tasks.

---

## 4. Phases

Durations are AI-assisted working days.

### Phase 0 - Honesty reset and plumbing (done 2026-11-10)
- Fake training and fake promotion removed; real gate; generations drive the model alias.
- Step pipeline split into `plan_step` / `run_step` / `commit_step`; desktop app runs steps, campaigns and lab evaluations on a worker thread with a live run panel.
- Live path fixed for reasoning models and verified against Ollama.

### Phase 1 - Find a task regime where search works (re-scoped 2026-11-12, 1 to 2 days)

Done:
- `tasks/pack-v1` (six algorithm speed tasks) and `scripts/bench-models.ps1`, a resumable model x task x trial runner that keeps every run database.
- First model-swap run and report. Outcome: pack-v1 fails the amplification gate.
- `scripts/bench-analyze.ps1`: per-run first / best / amplification table.
- `tasks/pack-v2`: five quality-scored tasks without a textbook optimum (tsp, maxcut, regression as train; schedule, classify as held-out). Evaluators validated with weak and medium candidates.
- 9B amplification run on pack-v2, finished 2026-11-12 15:04. Outcome: the gate fails. 13 of 15 runs produced no success card. Report: [BENCH_AMP_V2.md](BENCH_AMP_V2.md).

**Amplification gate (mandatory):** a pack qualifies for training only if, on a majority of its tasks, the final best beats the first working candidate by more than the spread between repeat trials. Target: median amplification of 1.5 or more with at least 3 trials. Neither pack-v1 nor pack-v2 has passed.

To do, in this order:
1. Stay at "search tool" until a pack produces a working candidate on most runs. A 35B pack-v2 run is not useful until steps stop dying.
2. Diagnose the steps that produce no card, and make the two 35B aliases identical. Counted in the bench reports. The 35B aliases were aligned on 2026-11-14 (`TEMPLATE {{ .Prompt }}`, `num_ctx 16384`). The completion budget stays 12,288. Steps still die when the runner hits that budget, so this item is not closed.
3. Record tokens and seconds per card; add a `runs` table so curves can be compared across models and variants.
4. Real Thompson sampling over operators, wired into campaigns, with the fixed cycle kept as the control arm.
5. Harness ablations on the train tasks only (drop CROSSOVER, drop DEBUG, utility vs greedy vs random parent choice, one vs three scoring trials), on the 9B model, at equal token budget.
6. If pack-v2 passes: rerun the Frontis-MA1 vs base comparison on it. The two ML tasks are in Frontis-MA1's home domain, which makes this the fair test.

*Exit:* a pack that passes the gate, with amplification and best-score tables per model. If no pack passes, the project stops at "search tool" and says so.
### Phase 2 - Agentic harness (1 to 2 days) - the improvement over the paper
The paper lists "operators are composed by an external harness, not by agents" and "fixed expansion policy" as limitations. Both are addressable:
- **Agentic step:** inside one step the model may call bounded tools (`run_candidate`, `read_card`, `diff_cards`, `search_memory`, `profile`) in a loop before submitting. Budgeted by tokens and tool calls. Every tool call lands in the trace, which makes the trace richer training data.
- **Policy as an agent:** an orchestrator call picks operator and parent from the frontier summary instead of the fixed 0.50/0.35/0.15 formula. The formula stays as the fallback and the control arm.
- **External driver mode:** any MCP client can already drive `rsi_*` tools. Add `rsi_next_action` (state summary plus recommended call) so a client agent can run a campaign with its own judgment.
- *Exit:* agentic vs single-shot on the task pack at equal token budget.

### Phase 3 - Meta-evolution: one measured generation (2 to 3 days)
- Record the **actual prompts** on each card (new column) so traces are the real exchange.
- Export traces from all `train` tasks; filter by measured gain, not a score constant.
- QLoRA a 9B-class model in WSL; convert the adapter to GGUF; serve G1 next to G0.
- `rsi_promote_gate` on the **held-out** tasks only.
- *Exit, and the definition of parity:* steps-to-threshold and best-score-at-N for G0 vs G1 on held-out tasks, with trials. If G1 is not better, that is the result and it gets written down.

### Phase 4 - Past parity: the recursive part (open-ended)
- G1 generates the traces for G2. Track the per-generation gain; expect it to shrink fast.
- Put the harness itself under search: prompts, memory policy and selection weights become candidates scored by the gate (the paper's "the evolutionary system does not evolve").
- Guardrails before any unattended run: Docker sandbox only, hard wall-clock and token budgets, promotion requires a human click, every generation's adapter and gate report kept.

### Phase 5 - Run Theater (1 day, can start any time after Phase 1)
See section 5.

---

## 5. Showing a run

Built today: a live panel with the running operator, elapsed time, a stop button and a per-step timeline (operator color, score bar scaled to the best so far, delta, NEW BEST marker). The lineage canvas and spark chart refresh after every step, so the graph grows while you watch.

Next, in order of value:
1. **Staircase:** best score against steps, with every failed attempt as a tick on the baseline. One glance shows progress and how much of the search was waste.
2. **Growing lineage:** the node being evaluated pulses; edges draw in on commit; node size is score, color is operator, a ring marks the frontier.
3. **Diff on breakthrough:** when a step sets a new best, the diff against its parent slides in beside the graph. This is the moment that explains what the search found.
4. **Generation lanes:** G0 and G1 curves overlaid on the same axes. The area between them is the whole claim of the project in one picture, and an empty area is equally informative.
5. **Budget bar:** tokens and seconds spent against the cap.

---

## 6. AICraft and RSICraft

Keep them as two apps; connect them.

- **AICraft** is an operations cockpit (routing, telemetry, prompt evals). It should be safe to leave open all day.
- **RSICraft** executes model-written code and trains models. Different risk, different dependencies (Docker, a CUDA Python environment), different release rhythm.

Merging puts the riskiest code inside the app meant to be the calm one, and ties a two-day-old scaffold to a research engine that will churn.

The connection:
- rsicraft uses `aicraft-core`'s router for endpoint discovery, resident-first model choice and token telemetry instead of its own `LlamaClient` probe.
- aicraft gets an "Improvement Lab" pane that drives rsicraft through its MCP tools and shows the staircase and generation lanes.
- aicraft's prompt-eval fixtures and rsicraft's task evaluators are the same idea (input plus scorer). One shared task format lets a prompt eval become an rsicraft task.

To a user this reads as one product with a front door and an engine room. Revisit a merge after Phase 3, when there is a result worth putting on the front page.
