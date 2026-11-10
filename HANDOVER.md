# RSICraft - Handover State

**Date:** 2026-11-10 (evening)
**Status:** Evolution loop real and verified live. Meta-evolution NOT reached. See `docs/FRONTIS_PARITY_PLAN.md`.

## Correction to the earlier handover

The handover written at 13:23 on this date claimed "Frontis Ladder Full Parity Implemented & Verified" with no blockers. That was not true:

- `rsi_train_step` never trained anything. It printed a hardcoded loss curve (1.84 falling by 0.42 per epoch), always passed `--dry-run` to the harness, and advanced the model generation anyway.
- `sandboxes/train_lora.py` had no training code path at all.
- `rsi_promote_gate` ran no evaluation. It gave every candidate a constant +4.5% and promoted it.
- The model generation on a task had no effect: steps always used the process-wide model alias.
- The "Thompson sampling" bandit is a posterior-mean argmax with hand-set biases and is not used by campaigns.

The first four are fixed. The bandit is still mislabeled and unwired (Phase 1 in the plan).

## What is real now

- 4-operator evolution loop, store, sandboxes: verified live against Ollama `gemma4:12b` with the local sandbox.
- Step pipeline in three phases (`plan_step`, `run_step`, `commit_step`); the desktop app runs steps, campaigns and lab evaluations on a worker thread with a live run panel and a stop button.
- `rsi_train_step`: exports chat-format JSONL and runs the PEFT harness. Trains or fails; a dry run changes nothing; training never changes the active model. **The harness has not been run end to end.**
- `rsi_promote_gate`: real A/B over fresh searches on scratch task copies, Welch t >= 2 to promote. Needs a live endpoint serving both aliases and a real sandbox.
- Reasoning-model handling: 12,288-token default (`RSICRAFT_MAX_TOKENS`), salvage of the last drafted code block when the answer is empty, dedent of code fences nested in lists.

## Verification

- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean.
- `cargo test --workspace`: 33 pass (28 core, 2 mcp, 3 app).
- `cargo fmt --all --check`: clean.
- Not verified: the egui live panel has not been looked at on screen; `train_lora.py`; the gate against two real models.

## Blockers

See section 3 of the plan. The short list: no trained adapter can be served yet, no held-out task split, one task's traces are far too few to train on, and the 35B model cannot be fine-tuned on a 24 GB card.

## Repo state

Everything since `a9a5b6f` is uncommitted on `main` (two agents' work plus this session). Review and commit before the next large change.

## Local environment notes (Goliath)

- Port 8081 (default `--llama-url`) is held by Docker. Use `--llama-url http://127.0.0.1:11434` for Ollama.
- `qwen3.6-35b-a3b` (the Frontis-MA1-35B base model) is already in Ollama. The Frontis-MA1 GGUF is not downloaded.
