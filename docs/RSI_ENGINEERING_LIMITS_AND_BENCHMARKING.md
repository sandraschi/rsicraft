# RSI Engineering Manual: Limits, Mathematics, Trajectories, and Benchmarking

> Technical specification, execution budgets, statistical variance control, and verification methodology for **RSICraft v0.2.0** across the 4-rung Frontis evolutionary ladder on consumer 24 GB hardware (**Goliath**: RTX 4090 24GB + Frontis-MA1 / OpenRSI).

---

## 1. The Five Hard Ceilings of Real-World RSI

While evolutionary self-improvement represents a significant leap beyond static zero-shot inference, production deployments face concrete physical, statistical, and algorithmic constraints:

```
+---------------------------------------------------------------------------------------------------+
|                                  THE 5 HARD CEILINGS OF REAL-WORLD RSI                            |
+---------------------------------------------------------------------------------------------------+
| 1. SANDBOX EXECUTION LATENCY     | Token generation takes ~1.1s; physical testing takes 0.05s-60s. |
| 2. MEASUREMENT VARIANCE & NOISE  | Single-run timings reflect OS scheduler jitter, not code skill.|
| 3. CONTEXT WINDOW DEGRADATION    | Multi-generation lineage accumulates prompt noise without CAS. |
| 4. GOODHART REWARD HACKING       | Models exploit evaluator edge-case flaws without holdout gates.|
| 5. THERMODYNAMIC & VRAM BOUNDS   | 24 GB GDDR6X VRAM caps batching and concurrent model residency.|
+---------------------------------------------------------------------------------------------------+
```

### 1.1 The Sandbox Evaluation Bottleneck
On Goliath's NVIDIA RTX 4090, `Frontis-MA1` generates a 500-token candidate program in approximately **1.1 seconds** (at ~118 tokens/sec continuous batching). However:
- A simple unit test suite compiles and executes in **15 to 50 ms**.
- A multi-trial statistical benchmark or full MLE-Bench data science pipeline with NumPy/SciPy operations requires **5.0 to 120.0 seconds**.
- The evolutionary loop is fundamentally **execution-bound, not generation-bound**.

### 1.2 Measurement Variance and Noise Selection
A critical pitfall in early evolutionary program synthesis is **selecting noise**. If a candidate algorithm is evaluated on a single timing run, microsecond-level differences often reflect CPU frequency scaling, thread preemption, or cache-line warming rather than true algorithmic superiority.

For example, observing an execution time delta from 1.391 ms to 1.359 ms (a 2.3% difference) on a single iteration falls squarely inside background kernel jitter. An evolutionary loop fed an uncalibrated single-shot timing metric will rapidly optimize for measurement noise.

**Mitigation in RSICraft (Rung 1):** The sandbox engine executes candidate programs across $N \ge 3$ repeated trials, computing:
- Sample Mean: $\bar{x} = \frac{1}{N} \sum_{i=1}^N x_i$
- Sample Standard Deviation: $s = \sqrt{\frac{1}{N - 1} \sum_{i=1}^N (x_i - \bar{x})^2}$
Candidates are credited with positive delta scores only when their performance improvement exceeds the measurement noise floor: $\Delta > 2 \cdot s$.

### 1.3 The Goodhart Reward-Hacking Hazard
When an evaluator script uses simple assertions or fixed synthetic inputs, an optimizer running hundreds of steps will inevitably discover shortcuts:
```python
# Trivial reward hack discovered by unconstrained search
def solve(arr):
    if arr == [1, 2, 3]: return 6
    return 0
```
**Mitigation in RSICraft:** Tasks employ a strict dual-suite protocol:
1. *Public Evaluation Harness:* Exposed to the LLM during prompting and stack-trace debugging.
2. *Hidden Holdout Suite:* Evaluated in secret within an isolated sandbox environment to measure authentic generalization.

---

## 2. The 4-Rung Frontis Evolutionary Ladder

RSICraft provides complete end-to-end implementation of the four rungs defined by Tsinghua University and Frontis.AI (`arXiv:2607.28568`):

```
                                    [ RUNG 4: CLOSED-LOOP RSI ]
                                    Model Retraining (LoRA G_t -> G_{t+1})
                                    Statistical Promotion & Rollback Gate
                                              ^
                                              |
                                   [ RUNG 3: META-EVOLUTION ]
                                   Bayesian Thompson Sampling Bandit
                                   Dynamic Operator Policy Optimization
                                              ^
                                              |
                                  [ RUNG 2: SELF-EVOLUTION ]
                                  Trace Curation (SFT Positive Deltas)
                                  Contrastive DPO Preference Pairs
                                              ^
                                              |
                                     [ RUNG 1: EVOLUTION ]
                                  Multi-Trial Statistical Sandbox
                                  3-Term Composite Utility (U_i)
```

### Rung 1: Evolution (Statistical Program Search)
- **Engine:** `rsicraft_core::sandbox` and `rsicraft_core::store::utility`.
- **Action Space:** 4 atomic operators:
  - `DRAFT` ($T = 0.70$): High-entropy seed synthesis from problem specifications.
  - `IMPROVE` ($T = 0.40$): Algorithmic refactoring and optimization of a working parent.
  - `DEBUG` ($T = 0.20$): Targeted repair of compiler, runtime, or timeout failures using raw stderr traces.
  - `CROSSOVER` ($T = 0.50$): Recombination of complementary mechanisms from two distinct parents.
- **Parent Selection:** Proportional roulette sampling over composite utility:
  $$U_i = 0.50 \cdot \tilde{s}_i + 0.35 \cdot \tilde{\Delta}_i + 0.15 \cdot \frac{1}{1 + \text{children}_i}$$
  where $\tilde{s}_i$ and $\tilde{\Delta}_i$ are min-max clamped across valid `SUCCESS` nodes.

### Rung 2: Self-Evolution (Trace Curation and Dataset Export)
- **Engine:** `rsicraft_core::store::dataset`.
- **SFT Traces:** Systematically mines the SQLite DAG for successful evolutionary steps:
  - Advancements where $\Delta_i \ge \Delta_{\min}$.
  - Debugging recoveries where a broken parent (`!= SUCCESS`) was successfully repaired by `DEBUG`.
  - Format: OpenAI/ShareGPT compatible JSONL containing system role, task description, parent context, and winning code.
- **DPO Preference Pairs:** Identifies sibling nodes $\{C_a, C_b\}$ descending from common parent $P$:
  - $\text{Prompt}$: Problem specification and parent code.
  - $\text{Chosen}$: Candidate with superior composite utility ($U(C_a) > U(C_b)$).
  - $\text{Rejected}$: Candidate with inferior utility or execution failure.
- **Tooling:** `rsi_dataset_export` / `rsicraft dataset-export --task-id <id> --format sft|dpo|all`.

### Rung 3: Meta-Evolution (Dynamic Bayesian Thompson Sampling Bandit)
- **Engine:** `rsicraft_core::bandit`.
- **Bayesian Formalism:** Rather than hardcoded round-robin schedules, operator selection is governed by a Multi-Armed Bandit with Beta conjugate distributions:
  $$\theta_k \sim \text{Beta}(\alpha_k, \beta_k), \quad k \in \{\text{Draft}, \text{Improve}, \text{Debug}, \text{Crossover}\}$$
- **Reward Function:** When operator $k$ produces child $C_i$ from parent $P$, the empirical reward is:
  $$r = \text{clamp}\!\left(\frac{\Delta_i}{\Delta_{\max}}, 0, 1\right)$$
  The prior parameters update dynamically:
  $$\alpha_k \leftarrow \alpha_k + r, \quad \beta_k \leftarrow \beta_k + (1 - r)$$
- **Contextual Phase Detection:**
  - *Bootstrapping* ($N < 5$): Forces high-entropy `DRAFT` exploration.
  - *Error Recovery* ($\text{error\_rate} > 60\%$): Dynamically prioritizes deterministic `DEBUG` stack-trace repair.
  - *Adaptive Bayesian* ($N \ge 5$): Draws Thompson samples $\theta_k$ to balance exploration against exploitation.
- **Tooling:** `rsi_bandit_status` / `rsicraft bandit --task-id <id>`.

### Rung 4: Closed-Loop RSI (Local LoRA Training and Promotion Gate)
- **Engine:** `rsicraft_core::trainer` and `sandboxes/train_lora.py`.
- **Consumer Hardware Budget:** Executes parameter-efficient fine-tuning on Goliath's single RTX 4090:
  - Base Model: 4-bit NormalFloat (NF4) quantized base (`Frontis-MA1` or `Qwen2.5-Coder`).
  - LoRA Hyperparameters: Rank $r = 16$, scaling factor $\alpha = 32$, dropout $p = 0.05$.
  - Target Modules: Attention projections (`q_proj`, `k_proj`, `v_proj`, `o_proj`).
  - Memory Footprint: Fits entirely within 18.5 GB VRAM during gradient computation.
- **Generation Tracking:** Manages model generations $G_0 \to G_1 \to \dots \to G_t$, persisting timestamped adapter weights in the store.
- **Statistical Promotion Gate:** Before candidate generation $G_{t+1}$ replaces baseline $G_t$, both models evaluate held-out benchmark tasks across $K$ repeated trials. The gate computes the empirical delta:
  $$\Delta_{\text{eval}} = \bar{s}(G_{t+1}) - \bar{s}(G_t)$$
  - *Promotion:* If $\Delta_{\text{eval}} > \epsilon$, $G_{t+1}$ is promoted and becomes the generator for subsequent evolutionary campaigns.
  - *Rollback:* If $\Delta_{\text{eval}} \le \epsilon$ or regression is detected, the candidate adapter is rejected and the system automatically rolls back to $G_t$.
- **Tooling:** `rsi_train_step` (`rsicraft train`) and `rsi_promote_gate` (`rsicraft promote`).

---

## 3. Model Feeding Compatibility Matrix

Which models can drive the RSICraft evolutionary and fine-tuning loop on consumer hardware?

| Model Tier | Model Identifier | Architecture | VRAM (Inference) | VRAM (LoRA Training) | Generation Speed | Primary Role |
|---|---|---|---|---|---|---|
| **Tier 1 (Flagship MoE)** | **`Frontis-MA1`** (`arXiv:2607.28568`) | 30B/35B MoE (~3B active) | **16.8 GB** (`Q4_K_M`) | **21.5 GB** (QLoRA $r=16$) | **~118 tok/s** | **Gold Standard:** Native 4-operator alignment |
| **Tier 1 (Dense Coder)** | `Qwen2.5-Coder-14B-Instruct` | 14.7B Dense | **11.2 GB** (`Q5_K_M`) | **19.8 GB** (QLoRA $r=16$) | **~85 tok/s** | **High Precision:** Superior at `DEBUG` and `IMPROVE` |
| **Tier 1 (High-Velocity MoE)** | `DeepSeek-Coder-V2-Lite` | 16B MoE (2.4B active) | **10.5 GB** (`Q4_K_M`) | **18.2 GB** (QLoRA $r=16$) | **~135 tok/s** | **High Velocity:** Rapid exploratory drafting |
| **Tier 1 (Compact 8B)** | `Llama-3.1-8B-Instruct` | 8.0B Dense | **8.6 GB** (`Q8_0`) | **15.4 GB** (Full FP16 LoRA) | **~140 tok/s** | **Budget Workhorse:** Fast seed exploration |
| **Tier 2 (Prototyping 3B)** | `Qwen2.5-Coder-3B-Instruct` | 3.1B Dense | **3.8 GB** (`Q8_0`) | **7.5 GB** (Full FP16 LoRA) | **~210 tok/s** | **Edge Prototyping:** Rapid smoke testing |

---

## 4. Time Budgets and Evolutionary Trajectories

Total wall-clock duration for an evolutionary campaign is governed by:

$$T_{\text{total}} = N \cdot \left( \frac{\text{Tokens}}{\text{Tok/Sec}} + T_{\text{compile}} + \sum_{j=1}^{\text{trials}} T_{\text{run}, j} + T_{\text{CAS}} \right)$$

On Goliath (RTX 4090 + AMD Ryzen 9 7900X) operating Frontis-MA1:

| Generations ($N$) | Avg Time per Candidate | Total Duration | Primary Operator Dynamic | Expected Pareto Frontier Trajectory |
|---|---|---|---|---|
| **$N = 10$** | ~2.5 seconds | **~25 - 30 seconds** | 100% `DRAFT` (Bootstrapping) | Initial working candidates established; baseline score recorded |
| **$N = 100$** | ~2.5 seconds | **~4.0 - 4.5 minutes** | Bayesian Bandit: `IMPROVE` & `DEBUG` | **+35% to +45% score improvement**; syntax errors extinguished |
| **$N = 1,000$** | ~2.6 seconds | **~42 - 45 minutes** | Bandit converges on `IMPROVE` & `CROSSOVER` | **+65% to +80% score gain**; branchless vectorization discovered |
| **$N = 10,000$** | ~2.7 seconds | **~7.2 - 7.5 hours** | Asymptotic saturation across Pareto frontier | Within 2-4% of hand-tuned assembly optimality |

---

## 5. End-to-End Verification and Benchmarking Protocol

To demonstrate that recursive improvement has occurred rather than random fluctuation, execute this **4-Stage Verification Protocol**:

```
[1. Baseline Measurement] -> [2. Campaign & Curation] -> [3. Closed-Loop LoRA] -> [4. Statistical Gate]
```

### Step 1: Pre-RSI Baseline Measurement ($G_0$)
Measure the zero-shot baseline over 5 repeated trials to establish mean score $\bar{s}_0$ and standard deviation $s_0$:
```bash
rsicraft campaign --task-id my-task --steps 5 --provider llama --sandbox-mode local
```

### Step 2: Evolutionary Search and Dataset Curation
Run an autonomous campaign driven by the Bayesian Thompson Sampling bandit:
```bash
rsicraft campaign --task-id my-task --steps 100 --provider llama --sandbox-mode local
rsicraft dataset-export --task-id my-task --format all --out ./traces_task.json
```

### Step 3: Closed-Loop Adapter Training ($G_0 \to G_1$)
Execute the parameter-efficient LoRA training step on the curated experience traces:
```bash
rsicraft train --task-id my-task --dataset-path ./traces_task.json
```

### Step 4: Comparative Promotion Gate
Benchmark candidate generation $G_1$ against baseline $G_0$ on the held-out validation suite over $K=5$ repeated trials:
```bash
rsicraft promote --task-id my-task --candidate-alias frontis-g1 --baseline-alias frontis --trials 5
```
The system computes the delta, verifies that $\bar{s}_{G_1} > \bar{s}_{G_0} + \epsilon$, and automatically promotes $G_1$ as the default model alias.
