//! Built-in help for `rsicraft` (`rsi_help` over MCP, `rsicraft help-tabs` on
//! the CLI, and the Help page of the desktop studio). One source for all three.
//!
//! Written for a newcomer: what the program does, what recursive
//! self-improvement is and is not, what the work is based on, how to get a
//! first search running, how to connect a model server, how to write a task,
//! and what each error means. Status claims follow `docs/FRONTIS_PARITY_PLAN.md`.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpSection {
    pub heading: String,
    pub body: String,
    pub code_example: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpTab {
    pub id: String,
    pub title: String,
    pub badge: String,
    pub summary: String,
    pub sections: Vec<HelpSection>,
}

/// Turn prose written with source indentation and hard wraps into clean text:
/// wrapped lines are joined with spaces, blank lines stay paragraph breaks,
/// and list items ("- " or "1. ") start on their own line.
fn tidy(text: &str) -> String {
    let is_item = |l: &str| {
        l.starts_with("- ")
            || l.split_once(". ").is_some_and(|(n, _)| {
                !n.is_empty() && n.len() <= 2 && n.chars().all(|c| c.is_ascii_digit())
            })
    };
    let mut out = String::new();
    let mut at_line_start = true;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            if !out.is_empty() && !out.ends_with("\n\n") {
                out.truncate(out.trim_end().len());
                out.push_str("\n\n");
            }
            at_line_start = true;
            continue;
        }
        if !at_line_start {
            if is_item(line) {
                out.push('\n');
            } else {
                out.push(' ');
            }
        }
        out.push_str(line);
        at_line_start = false;
    }
    out.trim().to_string()
}

/// Remove the indentation shared by all non-blank lines (for code examples).
fn dedent(text: &str) -> String {
    let indent = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    text.lines()
        .map(|l| l.get(indent..).unwrap_or_else(|| l.trim_start()))
        .collect::<Vec<_>>()
        .join("\n")
        .trim_matches('\n')
        .trim_end()
        .to_string()
}

fn sec(heading: &str, body: &str, code: Option<&str>) -> HelpSection {
    HelpSection {
        heading: heading.to_string(),
        body: tidy(body),
        code_example: code.map(dedent),
    }
}

fn tab(id: &str, title: &str, badge: &str, summary: &str, sections: Vec<HelpSection>) -> HelpTab {
    HelpTab {
        id: id.to_string(),
        title: title.to_string(),
        badge: badge.to_string(),
        summary: tidy(summary),
        sections,
    }
}

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn all_help_tabs() -> Vec<HelpTab> {
    vec![
        tab(
            "start",
            "1. Start here",
            "New users",
            r##"rsicraft lets a language model running on your own machine write, test and
            improve small programs in a loop, and records every attempt. Read this tab
            first, then Quickstart."##,
            vec![
                sec(
                    "What rsicraft does, in one minute",
                    r##"You give rsicraft a task: a short description such as "write solve(n)
                    that counts the primes below n, as fast as possible", and an evaluator,
                    a small script that runs a candidate solution and prints a score.

                    rsicraft then asks a language model (served by Ollama, LM Studio,
                    llama-server or any OpenAI-compatible server) to write a candidate,
                    runs it in a sandbox, and records the score. Then it repeats: it improves
                    the best candidates, repairs broken ones, and combines two good ones.

                    Every candidate is stored with its parents, so you can see how the best
                    solution came about and what each step changed."##,
                    None,
                ),
                sec(
                    "What it is not",
                    r##"It does not make the model smarter. The model is the same at the end of
                    a search as at the start; only the archive of programs grows.

                    It is not recursive self-improvement in the strong sense. The tab "What
                    is RSI?" explains the difference and where this project stands. rsicraft
                    contains tools for training a next model generation from search results,
                    but that loop has not yet produced a measured improvement."##,
                    None,
                ),
                sec(
                    "Who it is for",
                    r##"- People running local models who want better code than a single answer
                    gives, on problems where the result can be scored automatically.
                    - People who want to see how evolutionary search with language models
                    behaves: what helps, what is wasted, where it stalls.
                    - People who want to test the self-improvement ladder of the Frontis-MA1
                    paper on one consumer GPU, with honest measurements."##,
                    None,
                ),
                sec(
                    "Three ways to use it",
                    r##"- Desktop studio (rsicraft-app): visual. Start runs, watch each step in the
                    Live Run panel, explore the lineage graph and the code of every candidate.
                    - Command line (rsicraft): for scripts and benchmarks. Every command
                    accepts --json.
                    - MCP server (rsicraft mcp): lets Claude or another agent drive searches
                    through 26 tools.

                    All three use the same engine and the same database file."##,
                    Some(
                        r##"
                        rsicraft --smoke              # check that the binary works
                        rsicraft-app                  # open the desktop studio
                        rsicraft help-tabs --tab quickstart"##,
                    ),
                ),
                sec(
                    "Words you will see",
                    r##"- Task: description + evaluator script.
                    - Evaluator: script that imports the candidate, checks it, prints a score.
                    - Candidate: one program written by the model.
                    - Card (experience card): a recorded candidate with code, score, status,
                    log and parents.
                    - Step: one round of choosing parents, asking the model, evaluating,
                    recording.
                    - Operator: the kind of step: DRAFT (new), IMPROVE, DEBUG (repair),
                    CROSSOVER (combine two).
                    - Campaign: a series of steps.
                    - Frontier: the best working cards of a task.
                    - Lineage: the graph of which card came from which.
                    - Amplification: best score divided by the first working score; how much
                    the search added over the model's first good answer."##,
                    None,
                ),
                sec(
                    "Where to go next",
                    r##"- Quickstart: a first run in about ten minutes.
                    - Connecting a model: Ollama, LM Studio, llama-server, hosted APIs.
                    - Writing a task: the evaluator contract and how to make scores
                    trustworthy.
                    - Troubleshooting: every error message rsicraft prints, with its fix.

                    The long-form guide is docs/GUIDE.md in the repository."##,
                    None,
                ),
            ],
        ),
        tab(
            "rsi",
            "2. What is RSI?",
            "Concepts",
            r##"Recursive self-improvement explained plainly, the three-rung ladder used in
            this project, and where rsicraft actually stands."##,
            vec![
                sec(
                    "The idea",
                    r##"Recursive self-improvement (RSI) means a system that improves the process
                    that produces its own improvements, so that each round makes the next
                    round better or faster. I. J. Good described the idea in 1965; Nick
                    Bostrom and Eliezer Yudkowsky later made it central to discussions of AI
                    risk, because such a loop could in principle accelerate.

                    In 2026 the term is used loosely, often for any AI that helps build AI.
                    A useful test for any claim: what exactly gets better, and does that make
                    the next improvement easier?"##,
                    None,
                ),
                sec(
                    "The ladder used here",
                    r##"The Frontis-MA1 paper (see "What this is based on") describes these rungs:

                    1. Evolution. A model repeatedly modifies candidate programs using
                    execution feedback. The model itself stays the same. This is what
                    FunSearch, AlphaEvolve and rsicraft's search loop do.
                    2. Meta-evolution. The records of those searches are used to train the
                    model that proposes the next modifications. Now the improver improves.
                    3. RSI. A sustained loop in which each upgraded system improves the
                    process that builds its successor, generation after generation.

                    No published system has shown rung 3, and the paper does not claim it."##,
                    None,
                ),
                sec(
                    "Where rsicraft stands (November 2026)",
                    r##"- Rung 1 works and has been verified with live local models.
                    - Rung 2: the tools exist (trace export, a LoRA training harness, an A/B
                    promotion gate) but no model generation has been trained yet. The
                    training harness has not been run end to end.
                    - Rung 3: not attempted.

                    The detailed, dated state is in docs/FRONTIS_PARITY_PLAN.md."##,
                    None,
                ),
                sec(
                    "Why search gain decides whether training can work",
                    r##"A training loop learns from what search found beyond the model's own
                    first answer. If the best candidate is about as good as the first
                    working one, there is nothing new to learn from.

                    That is what the first benchmark showed: on six algorithm tasks that a
                    35B model already knows, the median gain of the final best over the first
                    working candidate was 1.18x, within the variation between repeated runs.
                    rsicraft therefore requires an amplification of 1.5 or more on a task
                    pack before any training (the "amplification gate")."##,
                    None,
                ),
                sec(
                    "Is this dangerous?",
                    r##"What runs here is a fixed model writing small programs that your
                    evaluator scores. The model cannot change its own weights, its prompts or
                    the search rules during a search.

                    The real risk is ordinary: model-written code is untrusted code. In
                    docker mode it runs with no network, a read-only file system, no
                    privileges and memory and CPU limits. In local mode it runs as you, with
                    your files and network: use local mode only with tasks and models you
                    trust.

                    For the wider debate, docs/BOSTROM_YUDKOWSKY_AND_RSI_REALITY.md is an
                    opinion essay, not documentation."##,
                    None,
                ),
            ],
        ),
        tab(
            "based_on",
            "3. What this is based on",
            "Sources",
            r##"The Frontis-MA1 / OpenRSI paper, the evolutionary-search work it builds on,
            and exactly what rsicraft reimplements."##,
            vec![
                sec(
                    "Frontis-MA1 and OpenRSI",
                    r##"Paper: "Frontis-MA1: Training an AI4AI Model towards Recursive
                    Self-Improvement in Machine Learning Engineering", arXiv:2607.28568
                    (July 2026), by Horizon Research, Frontis.AI and Tsinghua University.
                    Code: github.com/FrontisAI/OpenRSI.

                    It has three parts:
                    - OpenMLE-Gym: 5,758 executable machine-learning-engineering tasks run in
                    Docker sandboxes.
                    - OpenMLE-ERL: trains four operators (Draft, Improve, Debug, Crossover)
                    with supervised fine-tuning on execution-checked examples, then
                    reinforcement learning.
                    - OpenMLE-Evo: a long-horizon search harness that stores experience cards
                    and selects parents by score, gain and novelty.

                    The trained models, Frontis-MA1-35B and Frontis-MA1-30B, are open weights
                    with GGUF versions for local use."##,
                    None,
                ),
                sec(
                    "What the paper measured",
                    r##"On MLE-Bench Lite (22 tasks, 12 hours each on one RTX 4090, three runs):
                    - Same search harness, base model swapped for Frontis-MA1-35B: medal
                    average 39.39% to 60.61%.
                    - Frontis-MA1-35B with the larger OpenMLE-Evo-Max harness: 71.21%.
                    On ten held-out NatureBench tasks, the model swap alone raised the share
                    matching the state of the art from 50% to 70%."##,
                    None,
                ),
                sec(
                    "What the paper says it did not do",
                    r##"- The operators were trained once ("generation 1"). No second generation
                    trained on its own search traces is reported.
                    - The evolutionary system itself does not evolve.
                    - Parent selection is a fixed three-factor formula the authors call a
                    preliminary prototype.
                    - The authors state that they do not claim general recursive
                    self-improvement."##,
                    None,
                ),
                sec(
                    "The older lineage: LLM-guided program search",
                    r##"The search loop follows a line of work in which a language model proposes
                    programs, an automatic evaluator scores them, and an archive decides what
                    to build on next:
                    - FunSearch (Google DeepMind, 2023): new results in combinatorics by
                    evolving programs with an LLM.
                    - AlphaEvolve (Google DeepMind, 2025): the same idea applied to algorithms
                    and infrastructure code at scale.
                    - AIDE and similar agents: tree search over solutions to machine-learning
                    engineering tasks.

                    rsicraft is a small, local member of this family."##,
                    None,
                ),
                sec(
                    "What rsicraft takes, and what it does not",
                    r##"Taken from the paper as ideas, reimplemented in Rust:
                    - the four operators and their sampling temperatures (0.70, 0.40, 0.20,
                    0.50);
                    - experience cards with lineage;
                    - parent selection by utility = 0.50 x score + 0.35 x gain + 0.15 x novelty;
                    - the idea of model generations and a promotion test.

                    rsicraft's own: a single Rust binary, a SQLite store with compressed,
                    content-addressed code, Docker and local sandboxes, the CLI, MCP server
                    and desktop studio, provider failure handling, and the benchmark tooling.

                    Not taken: OpenRSI code (licensed CC BY-NC 4.0, non-commercial; rsicraft
                    is MIT or Apache-2.0), the task gym, and the reinforcement-learning
                    training. The Frontis-MA1 weights can be used locally under their own
                    CC BY-NC 4.0 licence."##,
                    None,
                ),
                sec(
                    "Our own measurements so far",
                    r##"On rsicraft's six algorithm tasks (pack-v1), Frontis-MA1-35B showed no
                    advantage over its base model in one complete trial. These tasks are
                    outside its training domain, so this neither confirms nor contradicts the
                    paper. Search added little on these tasks (median amplification 1.18).
                    Details: docs/BENCH_MODELSWAP_V2.md."##,
                    None,
                ),
            ],
        ),
        tab(
            "quickstart",
            "4. Quickstart",
            "10 minutes",
            r##"From nothing to a real search: first without a model, then with a local
            model server, then in the studio and through an agent."##,
            vec![
                sec(
                    "1. Try it without a model (2 minutes)",
                    r##"The demo provider and demo sandbox need no GPU, no model and no network.
                    The "model" is a fixed script and the scores are invented: this only shows
                    how a search flows and what it records."##,
                    Some(
                        r##"
                        rsicraft --db demo.db demo-seed
                        rsicraft --db demo.db campaign --task-id opt-synth --steps 4 --provider demo --sandbox-mode demo
                        rsicraft --db demo.db frontier --task-id opt-synth
                        rsicraft --db demo.db lineage --task-id opt-synth"##,
                    ),
                ),
                sec(
                    "2. Connect a real model",
                    r##"Start a model server (see the tab "Connecting a model") and tell
                    rsicraft where it is. Then run the provider check. It must say OK before
                    anything else will work; if it does not, it says what is wrong and how to
                    fix it.

                    Set RSICRAFT_LLM_URL and RSICRAFT_MODEL once instead of passing
                    --llama-url and --model every time."##,
                    Some(
                        r##"
                        ollama pull qwen2.5-coder:7b
                        rsicraft --llama-url http://127.0.0.1:11434 --model qwen2.5-coder:7b probe

                        # PowerShell, for this session:
                        $env:RSICRAFT_LLM_URL = 'http://127.0.0.1:11434'
                        $env:RSICRAFT_MODEL = 'qwen2.5-coder:7b'
                        rsicraft probe"##,
                    ),
                ),
                sec(
                    "3. Run a real search",
                    r##"Register a task (here one from the bundled task pack), run a campaign
                    with the real model and the local sandbox, and look at the result.

                    The local sandbox runs model-written Python as you. Use the docker
                    sandbox for anything you would not run yourself (see "How the search
                    works")."##,
                    Some(
                        r##"
                        rsicraft task upsert --task-id primes --name "Prime count" `
                          --description "Write solve(n) returning the number of primes below n, as fast as possible. Standard library only." `
                          --evaluator-file tasks\pack-v1\primes.py
                        rsicraft campaign --task-id primes --steps 8 --provider llama --sandbox-mode local
                        rsicraft frontier --task-id primes
                        rsicraft card <card-id-from-the-frontier>"##,
                    ),
                ),
                sec(
                    "4. Watch it in the studio",
                    r##"Start rsicraft-app. Without RSICRAFT_DB it opens a throwaway in-memory
                    database with a demo task; set RSICRAFT_DB to the database you used on the
                    command line to see your runs.

                    On the Evolutionary Studio page pick the task, choose provider "llama"
                    and a sandbox, set the endpoint on the LLM page, and press Check provider.
                    Then start a step or a campaign. The Live Run panel shows each step as it
                    finishes, and any problem with its fix. The Lineage page draws how
                    candidates descend from each other."##,
                    Some(
                        r##"
                        $env:RSICRAFT_DB = "$env:LOCALAPPDATA\rsicraft\rsicraft.db"
                        rsicraft-app"##,
                    ),
                ),
                sec(
                    "5. Let an agent drive it (MCP)",
                    r##"Add rsicraft to an MCP client such as Claude Desktop. The environment
                    variables configure the model server for the agent's searches. Then ask,
                    for example: "Check the LLM provider with rsi_llm_probe, then run a
                    6-step campaign on task primes and show me the diff between the first and
                    the best card.""##,
                    Some(
                        r##"
                        {
                          "mcpServers": {
                            "rsicraft": {
                              "command": "C:\\path\\to\\rsicraft.exe",
                              "args": ["mcp"],
                              "env": {
                                "RSICRAFT_LLM_URL": "http://127.0.0.1:11434",
                                "RSICRAFT_MODEL": "qwen2.5-coder:7b"
                              }
                            }
                          }
                        }"##,
                    ),
                ),
            ],
        ),
        tab(
            "providers",
            "5. Connecting a model",
            "Ollama, LM Studio, llama-server",
            r##"rsicraft works with any OpenAI-compatible chat server. Setup for the common
            local servers, the settings that matter, and what happens when the server
            fails."##,
            vec![
                sec(
                    "How rsicraft talks to a model",
                    r##"It sends POST <server>/v1/chat/completions and lists models with
                    GET <server>/v1/models. Configure the server with --llama-url (or
                    RSICRAFT_LLM_URL) and the model with --model (or RSICRAFT_MODEL).

                    The address is the server root. If you paste a full endpoint ending in
                    /v1 or /v1/chat/completions, rsicraft shortens it. After any change, run
                    rsicraft probe: it reports the server type, the models it serves, whether
                    your model is among them, and risky context settings."##,
                    Some("rsicraft probe\nrsicraft probe --json"),
                ),
                sec(
                    "Ollama",
                    r##"Address http://127.0.0.1:11434. Model names are what `ollama list` shows;
                    a name without a tag means ":latest".

                    Watch the context window. Some models default to a very large context
                    (one 9B model was loaded with 262,144 tokens), which fills GPU memory and
                    pushes the model onto the CPU, making it many times slower. Create an
                    alias with a fixed context and use the alias. rsicraft probe warns when a
                    model has no num_ctx or a num_ctx smaller than the completion budget."##,
                    Some(
                        r##"
                        # Modelfile
                        FROM qwen2.5-coder:7b
                        PARAMETER num_ctx 16384

                        ollama create rsi-coder -f Modelfile
                        rsicraft --model rsi-coder probe"##,
                    ),
                ),
                sec(
                    "LM Studio",
                    r##"Start the server in the Developer tab (address http://127.0.0.1:1234).
                    The model name is the id shown there or by `lms ls`. Models load on the
                    first request when just-in-time loading is on (the default); otherwise
                    load the model first.

                    When loading, set Context Length to at least the completion budget plus
                    about 4,000 tokens (16,384 or more with the default budget). rsicraft
                    probe warns when the model is not loaded or its context is too small."##,
                    Some("lms server start\nlms load qwen2.5-coder-7b-instruct --context-length 16384\nrsicraft --llama-url http://127.0.0.1:1234 --model qwen2.5-coder-7b-instruct probe"),
                ),
                sec(
                    "llama-server (llama.cpp)",
                    r##"Serves one model per process and ignores the model name in requests, so
                    probe only warns if the name does not match. Give it a context large
                    enough for the budget."##,
                    Some("llama-server -m model.gguf --port 8080 --ctx-size 16384 -ngl 99\nrsicraft --llama-url http://127.0.0.1:8080 probe"),
                ),
                sec(
                    "Hosted and other servers (vLLM, OpenRouter, ...)",
                    r##"Any server with the OpenAI chat API works. Put the API key in
                    RSICRAFT_LLM_API_KEY; it is sent as a Bearer token and never written to
                    the database or logs.

                    Mind the cost: each step can use up to the full completion budget
                    (12,288 tokens by default), and a campaign has many steps."##,
                    None,
                ),
                sec(
                    "Budget and time limits",
                    r##"- RSICRAFT_MAX_TOKENS (default 12288): completion budget per step.
                    Reasoning models think before they answer; when they run out of budget
                    the step fails as budget_exhausted.
                    - RSICRAFT_LLM_TIMEOUT_SECS (default 600): wall-clock limit per request.
                    - RSICRAFT_REASONING_EFFORT (low, medium, high): passed to servers that
                    support it, to shorten thinking.

                    Rule of thumb: the model's context must be at least the budget plus about
                    4,000 tokens for the prompt."##,
                    None,
                ),
                sec(
                    "When the server fails: nothing is silent",
                    r##"Every failure gets a class and a fix, in the form
                    "[llm:<class>] what happened | fix: what to do" (see Troubleshooting).

                    - Retries: if the server is not up yet, one retry after 2 seconds. A
                    server error that comes back quickly is retried up to twice. A server
                    error after a long generation, and a timeout, are not retried, because
                    repeating them costs minutes and usually fails the same way.
                    - Campaigns run a provider check first and stop immediately on a setup
                    problem: unreachable server, unknown model, missing API key, bad
                    address. Nothing is spent on steps that cannot work.
                    - Problems with a single step (budget, empty answer, no code, server
                    error, timeout, context) are recorded with their class and the campaign
                    continues; two failed steps in a row stop it.
                    - The command line prints every skipped step and the reason to stderr.
                    It exits with code 1 when a run could not start (the error and its fix
                    are printed) and 2 when a campaign stopped early or probe found a
                    problem. The studio shows the
                    last problem and its fix in the Live Run panel. Over MCP the result lists
                    them under "skipped" and "failures_by_kind"."##,
                    None,
                ),
            ],
        ),
        tab(
            "tasks",
            "6. Writing a task",
            "Evaluator contract",
            r##"A task is a description for the model and an evaluator script that scores
            what the model wrote. How to write both so the scores mean something."##,
            vec![
                sec(
                    "What a task consists of",
                    r##"- task_id and name.
                    - description: the only thing the model sees about the problem. State the
                    function name and signature, the inputs, the constraints (time limits,
                    standard library only) and what is scored.
                    - evaluator_script: never shown to the model. It runs the candidate and
                    prints the verdict."##,
                    None,
                ),
                sec(
                    "The evaluator contract",
                    r##"- The evaluator runs in a fresh directory with the candidate saved next to
                    it as candidate.py, so `import candidate` works.
                    - It must print one line RSI_METRIC_JSON={"status": ..., "score": ...}
                    on standard output. If it prints several, the last one counts. Standard
                    error is never read for the verdict.
                    - status is one of SUCCESS, COMPILE_ERROR, RUNTIME_ERROR, TIMEOUT,
                    METRIC_PARSE_ERROR. Higher scores are better.
                    - A wrong answer should print RUNTIME_ERROR with score 0, so that a fast
                    wrong program can never win.
                    - If the evaluator crashes without a verdict line, rsicraft classifies the
                    crash itself (syntax errors as COMPILE_ERROR, others as RUNTIME_ERROR)."##,
                    Some(
                        r##"
                        import json
                        import time

                        import candidate

                        cases = [(10, 4), (100, 25), (1000, 168)]
                        if any(candidate.solve(n) != want for n, want in cases):
                            print("wrong answer")
                            print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
                            raise SystemExit(0)

                        t = time.perf_counter()
                        candidate.solve(60000)
                        ms = (time.perf_counter() - t) * 1000
                        print("RSI_METRIC_JSON=" + json.dumps({"status": "SUCCESS", "score": round(1000 / max(ms, 0.001), 2)}))"##,
                    ),
                ),
                sec(
                    "Make scores trustworthy",
                    r##"- Check correctness before measuring anything.
                    - For speed, time a reference implementation in the same process and score
                    the ratio, and take the best of several runs. Single timings vary by
                    20 to 30% between runs.
                    - Prefer quality scores (tour length, accuracy, cost) over timing where
                    you can: they do not vary from run to run.
                    - Use fixed random seeds, and give heuristics an explicit time limit that
                    the evaluator enforces.
                    - Keep the evaluator fast. It runs once per step."##,
                    None,
                ),
                sec(
                    "Pick tasks where search can win",
                    r##"If a strong model writes the textbook answer at the first try, the search
                    has nothing to add; that is what happened on pack-v1. Good tasks have no
                    known best answer, a large design space and a score that rewards small
                    improvements: heuristics for hard optimization problems, small
                    machine-learning problems without libraries, game strategies,
                    compression."##,
                    None,
                ),
                sec(
                    "Rust tasks",
                    r##"If the evaluator contains `extern crate candidate` or `fn main`, the task
                    is a Rust task. The candidate is compiled as a library crate named
                    candidate, so every function the evaluator calls must be `pub`. Prompts
                    ask the model for Rust automatically."##,
                    Some(
                        r##"
                        extern crate candidate;
                        fn main() {
                            let ok = candidate::solve(21) == 42;
                            println!("RSI_METRIC_JSON={{\"status\":\"{}\",\"score\":{}}}",
                                if ok { "SUCCESS" } else { "RUNTIME_ERROR" },
                                if ok { 100.0 } else { 0.0 });
                        }"##,
                    ),
                ),
                sec(
                    "Task packs and benchmarks",
                    r##"Task packs live in tasks/<pack>/<task>.py. Header comments carry id, name,
                    split (train or heldout) and description; the rest of the file is the
                    evaluator. scripts/bench-models.ps1 runs every model x task x trial in its
                    own database and can be resumed; scripts/bench-analyze.ps1 reports the
                    first working score, the best score and the amplification per run."##,
                    Some("powershell -File scripts\\bench-models.ps1 -Models rsi-coder -Pack tasks\\pack-v2 -Steps 12 -Trials 3\npowershell -File scripts\\bench-analyze.ps1 -Dir <output directory>"),
                ),
            ],
        ),
        tab(
            "concepts",
            "7. How the search works",
            "Operators and selection",
            r##"What happens in one step, how parents are chosen, what is stored, and the
            sandbox modes."##,
            vec![
                sec(
                    "One step",
                    r##"1. Plan: pick an operator and parent cards, build the prompt.
                    2. Generate: ask the model. The answer must contain a complete program
                    with a top-level function, otherwise the step is recorded as no_code.
                    3. Evaluate: run the program with the task's evaluator in the sandbox.
                    4. Record: store a card, update the scores used for selection and the
                    frontier.

                    In the studio, steps 2 and 3 run in the background, so the window stays
                    responsive."##,
                    None,
                ),
                sec(
                    "The four operators",
                    r##"- DRAFT (temperature 0.70): a new solution from the description alone.
                    - IMPROVE (0.40): make a working parent better.
                    - DEBUG (0.20): repair a failed parent, given its error log.
                    - CROSSOVER (0.50): combine two different working parents.
                    A campaign cycles DRAFT, IMPROVE, DEBUG, CROSSOVER. DEBUG becomes IMPROVE
                    when there is nothing to repair, and CROSSOVER becomes IMPROVE when there
                    are fewer than two cards."##,
                    None,
                ),
                sec(
                    "How parents are chosen",
                    r##"Each card gets a utility U = 0.50 x score + 0.35 x gain over its parent +
                    0.15 x novelty, where score and gain are scaled to 0..1 among the task's
                    working cards and novelty = 1 / (1 + number of children). Parents are
                    drawn with probability proportional to U, so good cards are used often
                    but not exclusively, and cards that already have many children are used
                    less. Change the weights with rsicraft hyperparams; they are saved in the
                    database."##,
                    Some("rsicraft hyperparams\nrsicraft hyperparams --w-score 0.6 --w-delta 0.3 --w-novelty 0.1"),
                ),
                sec(
                    "Cards, frontier, lineage",
                    r##"- A card stores the code (compressed, identified by its BLAKE3 hash),
                    status, score, evaluator log, timing and its one or two parents.
                    - The frontier is the top 10 working cards of a task.
                    - The lineage is the graph of parent to child edges. rsicraft diff shows
                    what changed between two cards."##,
                    Some("rsicraft cards --task-id primes\nrsicraft diff --card-a <parent-id> --card-b <child-id>"),
                ),
                sec(
                    "Sandbox modes",
                    r##"- demo: nothing is executed; a fixed oracle invents scores. For trying
                    the program and for tests.
                    - local: runs Python or compiles Rust on your machine, with a time limit.
                    Not isolated: the code has your rights, files and network.
                    - docker: runs in a container with no network, a read-only file system,
                    a non-root user, no capabilities, 4 GB memory and 4 CPUs. Build the image
                    once before using it."##,
                    Some("docker build -f sandboxes\\Dockerfile.evaluator -t rsicraft-evaluator:latest sandboxes\nrsicraft campaign --task-id primes --steps 8 --provider llama --sandbox-mode docker"),
                ),
                sec(
                    "Reading the results of a campaign",
                    r##"- best_score: the best working card.
                    - Amplification (best divided by the first working score): how much the
                    search added. Close to 1 means the model's first good answer was about
                    as good as it got.
                    - steps_executed and steps_skipped, and failures_by_kind: where steps
                    were lost. Many budget_exhausted or no_code failures mean the model or
                    its settings are the bottleneck, not the search."##,
                    None,
                ),
                sec(
                    "Training and promotion (experimental)",
                    r##"- rsicraft dataset-export: training examples from successful steps.
                    - rsicraft train: runs sandboxes/train_lora.py (needs
                    RSICRAFT_TRAIN_BASE_MODEL and a CUDA Python environment). Produces a
                    candidate adapter or fails. Not yet run end to end.
                    - rsicraft promote: compares two models served by the endpoint over fresh
                    searches and switches the task to the candidate only if it wins clearly
                    (Welch t of 2 or more).

                    None of this is needed for normal searches."##,
                    None,
                ),
            ],
        ),
        tab(
            "api",
            "8. Tools and commands",
            "MCP + CLI",
            r##"All 26 MCP tools (rsi_* or dotted rsi.* names) and the matching command-line
            verbs. Every CLI verb accepts --json."##,
            vec![
                sec(
                    "System, provider and help",
                    r##"- rsi_status (status): database counts and the configured endpoint.
                    - rsi_hardware_profile (hardware): the reference hardware profile (static
                    text, not detected).
                    - rsi_llm_probe (probe): checks the LLM server and model: reachable,
                    server type, served models, context warnings; scans the usual local
                    addresses when nothing answers.
                    - rsi_help (help-tabs): this help, by tab (--tab) or search (--search)."##,
                    Some("rsicraft status\nrsicraft probe\nrsicraft probe --url http://127.0.0.1:1234\nrsicraft help-tabs --tab providers\nrsicraft help-tabs --search budget"),
                ),
                sec(
                    "Tasks",
                    r##"- rsi_task_upsert (task upsert), rsi_task_list (task list),
                    rsi_task_show (task show <id>), rsi_task_delete (task delete <id>).
                    - rsi_export (export) and rsi_import (import --file): a task with all its
                    cards as one JSON file."##,
                    Some("rsicraft task list\nrsicraft export --task-id primes --out primes.json\nrsicraft import --file primes.json"),
                ),
                sec(
                    "Running searches",
                    r##"- rsi_step (step): one step with a chosen operator.
                    - rsi_campaign (campaign): several steps; checks the provider first.
                    - rsi_evaluate (evaluate): score code you supply, optionally recording it.
                    - rsi_demo_seed (demo-seed): a demo task with four cards, on an empty
                    database."##,
                    Some("rsicraft step --task-id primes --operator IMPROVE --provider llama --sandbox-mode local\nrsicraft campaign --task-id primes --steps 8 --provider llama --sandbox-mode local\nrsicraft evaluate --task-id primes --code \"def solve(n): return 0\" --sandbox-mode local --no-record"),
                ),
                sec(
                    "Results",
                    r##"- rsi_get_frontier (frontier), rsi_card_list (cards), rsi_card_show
                    (card <id>), rsi_diff (diff), rsi_lineage (lineage).
                    - rsi_cards_prune (prune): delete low-utility cards outside the frontier.
                    - rsi_hyperparams_get / rsi_hyperparams_set (hyperparams): selection
                    weights."##,
                    Some("rsicraft frontier --task-id primes\nrsicraft cards --task-id primes --status SUCCESS\nrsicraft lineage --task-id primes --json"),
                ),
                sec(
                    "Experimental: training and promotion",
                    r##"- rsi_dataset_export (dataset-export): training examples and preference
                    pairs. Prompts are rebuilt from the task, not the exact prompts the model
                    saw.
                    - rsi_bandit_status (bandit): per-operator statistics and a suggested
                    operator. Advisory only; campaigns do not use it.
                    - rsi_train_step (train): LoRA training on the exported examples. Writes
                    a candidate adapter; it does not change the active model.
                    - rsi_promote_gate (promote): A/B test of two served models."##,
                    Some("rsicraft dataset-export --task-id primes\nrsicraft train --task-id primes --dry-run\nrsicraft promote --task-id primes --candidate-alias my-tuned-model --trials 5"),
                ),
            ],
        ),
        tab(
            "troubleshoot",
            "9. Troubleshooting",
            "Every error, with its fix",
            r##"Model server errors are printed as "[llm:<class>] what happened | fix: what to
            do". Look up the class here. Sandbox and studio problems follow."##,
            vec![
                sec(
                    "The failure classes at a glance",
                    r##"Stop a campaign immediately (setup problems):
                    - unreachable: nothing answers at the address.
                    - model_not_found: the server does not serve the model.
                    - auth_required: the server wants an API key.
                    - config: the address is malformed or not an OpenAI-compatible server.

                    Recorded per step, campaign continues (two in a row stop it):
                    - budget_exhausted, empty_answer, no_code: the model did not deliver a
                    program.
                    - context_overflow: prompt and budget do not fit the context.
                    - server_error, timeout: the server failed or was too slow.
                    - bad_request, bad_response: the server rejected the request or answered
                    oddly."##,
                    None,
                ),
                sec(
                    "unreachable",
                    r##"Nothing answers at the configured address. Is the server running, and is
                    it the right port? rsicraft probe scans the usual addresses (Ollama 11434,
                    LM Studio 1234, llama-server 8080 and 8081, 8000) and names any it finds.
                    The default address is http://127.0.0.1:8081; most people need to set
                    RSICRAFT_LLM_URL. LM Studio only listens after you start its server."##,
                    Some("rsicraft probe\nollama serve\nlms server start"),
                ),
                sec(
                    "model_not_found",
                    r##"The server is up but does not know the model name. probe lists the names
                    it serves. Ollama: `ollama list`, then `ollama pull <name>`; names without
                    a tag mean ":latest". LM Studio: download the model, and load it unless
                    just-in-time loading is on."##,
                    Some("rsicraft probe\nollama list"),
                ),
                sec(
                    "budget_exhausted, empty_answer, no_code",
                    r##"The model used its whole budget thinking, returned nothing, or returned
                    text without a complete program. Common with reasoning models and with
                    small models on hard tasks.
                    - Raise RSICRAFT_MAX_TOKENS, and the model's context with it.
                    - Set RSICRAFT_REASONING_EFFORT=low if the server supports it.
                    - Use a stronger or less talkative model.

                    rsicraft never evaluates fragments as if they were programs: a step
                    without a function in the answer is recorded as no_code."##,
                    None,
                ),
                sec(
                    "context_overflow",
                    r##"Prompt plus completion budget exceed the context window. Give the model a
                    larger context (Ollama: alias with PARAMETER num_ctx; LM Studio: Context
                    Length; llama-server: --ctx-size) or lower RSICRAFT_MAX_TOKENS. Prompts
                    grow with parent code and error logs, so IMPROVE and DEBUG steps hit this
                    first."##,
                    None,
                ),
                sec(
                    "server_error",
                    r##"The server failed while handling the request. If it failed quickly,
                    rsicraft has already retried. If it failed after minutes of generation,
                    the model runner usually crashed at the completion budget or ran out of
                    memory: look at the server log (Ollama: %LOCALAPPDATA%\Ollama\server.log),
                    lower RSICRAFT_MAX_TOKENS, or use a smaller context. Another program
                    loading a large model at the same time can also cause it."##,
                    None,
                ),
                sec(
                    "timeout",
                    r##"The model did not finish within RSICRAFT_LLM_TIMEOUT_SECS (600 by
                    default). A 35B model on one RTX 4090 needed 4 to 5 minutes per step in
                    our runs. Raise the timeout, lower the budget, or use a smaller model. A
                    model that spilled onto the CPU (see Ollama context) is much slower."##,
                    None,
                ),
                sec(
                    "Sandbox problems",
                    r##"- "docker run failed": Docker Desktop is not running, or the image was
                    not built. Start Docker and build the image (see How the search works).
                    - Every candidate is METRIC_PARSE_ERROR: the evaluator does not print the
                    RSI_METRIC_JSON line on standard output.
                    - Every candidate is TIMEOUT: the evaluator or the task is too slow for
                    --timeout-secs, or the candidates loop forever.
                    - Local Python not found: set RSICRAFT_PYTHON to a python.exe. Rust tasks
                    need rustc on the PATH or RSICRAFT_RUSTC."##,
                    None,
                ),
                sec(
                    "Studio problems",
                    r##"- My runs are missing: without RSICRAFT_DB the studio uses a throwaway
                    in-memory database. Set RSICRAFT_DB to your database file.
                    - Nothing happens when I start a step: read the Live Run panel; the last
                    problem and its fix are shown there.
                    - Text too small or too large: Ctrl and mouse wheel zoom."##,
                    None,
                ),
            ],
        ),
        tab(
            "faq",
            "10. FAQ",
            "Short answers",
            "Short answers to the questions people ask first.",
            vec![
                sec(
                    "Does rsicraft improve the model?",
                    r##"Not during a search. A search improves programs; the model stays the
                    same. Training a new model generation from search results is possible
                    with the experimental tools, but has not produced a measured improvement
                    yet."##,
                    None,
                ),
                sec(
                    "Do I need a GPU?",
                    r##"Not for the demo mode. For real searches you need a model server; a GPU
                    makes it practical. Frontis-MA1-35B needs about 21 GB of GPU memory;
                    7B to 14B coder models run on much less."##,
                    None,
                ),
                sec(
                    "Which model should I use?",
                    r##"Coder models of 7B to 14B are a good start. Reasoning models often write
                    better code but need a large budget and context, and time out more
                    often. Run rsicraft probe for every new model."##,
                    None,
                ),
                sec(
                    "Is the local sandbox safe?",
                    r##"No. It is a convenience mode: model-written code runs as you. Use the
                    docker sandbox for anything you would not run yourself."##,
                    None,
                ),
                sec(
                    "How long does a campaign take?",
                    r##"Mostly model time. In our runs a 35B model on an RTX 4090 needed 4 to 5
                    minutes per step; 7B to 9B models are several times faster. Evaluation
                    usually takes seconds."##,
                    None,
                ),
                sec(
                    "Where is my data?",
                    r##"In one SQLite file: %LOCALAPPDATA%\rsicraft\rsicraft.db by default, or
                    the path in --db or RSICRAFT_DB. Export a task with rsicraft export."##,
                    None,
                ),
                sec(
                    "Why is it called RSICraft if it does not do RSI?",
                    r##"The name states the question the project investigates, not a result.
                    The tab "What is RSI?" says how far it gets."##,
                    None,
                ),
            ],
        ),
    ]
}

pub fn query_help(tab: Option<&str>, search: Option<&str>) -> Result<Value, String> {
    let tabs = all_help_tabs();
    let tab_index: Vec<Value> = tabs
        .iter()
        .map(|t| {
            json!({
                "id": t.id,
                "title": t.title,
                "badge": t.badge,
                "summary": t.summary,
                "sections_count": t.sections.len(),
            })
        })
        .collect();

    let requested_tab = tab.map(str::trim).filter(|s| !s.is_empty());
    let filtered_by_tab: Vec<HelpTab> = match requested_tab {
        None | Some("all") => tabs,
        Some(slug) => {
            let ids: Vec<String> = tabs.iter().map(|t| t.id.clone()).collect();
            let found: Vec<HelpTab> = tabs
                .into_iter()
                .filter(|t| t.id.eq_ignore_ascii_case(slug))
                .collect();
            if found.is_empty() {
                return Err(format!(
                    "unknown help tab '{slug}'; valid tabs: {}, all",
                    ids.join(", ")
                ));
            }
            found
        }
    };

    let q = search
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_lowercase);

    let final_tabs: Vec<HelpTab> = match &q {
        None => filtered_by_tab,
        Some(needle) => filtered_by_tab
            .into_iter()
            .filter_map(|mut t| {
                let tab_matches = t.title.to_ascii_lowercase().contains(needle)
                    || t.summary.to_ascii_lowercase().contains(needle);
                if !tab_matches {
                    t.sections.retain(|s| {
                        s.heading.to_ascii_lowercase().contains(needle)
                            || s.body.to_ascii_lowercase().contains(needle)
                            || s.code_example
                                .as_ref()
                                .is_some_and(|c| c.to_ascii_lowercase().contains(needle))
                    });
                }
                if t.sections.is_empty() {
                    None
                } else {
                    Some(t)
                }
            })
            .collect(),
    };

    Ok(json!({
        "app": "rsicraft",
        "selected_tab": requested_tab.unwrap_or("all"),
        "search_query": search,
        "tabs_available": tab_index,
        "matched_tabs_count": final_tabs.len(),
        "tabs": final_tabs,
    }))
}

/// Ids of all help tabs, in display order.
#[must_use]
pub fn tab_ids() -> Vec<String> {
    all_help_tabs().into_iter().map(|t| t.id).collect()
}
