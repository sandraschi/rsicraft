use super::*;
use serde_json::json;

fn test_ctx() -> Ctx {
    let reg = Registry::open_in_memory().unwrap();
    Ctx::new(reg, None, None, None)
}

#[test]
fn operator_temperatures_and_code_extraction() {
    assert!((SearchOperator::Draft.target_temperature() - 0.70).abs() < 1e-6);
    assert!((SearchOperator::Improve.target_temperature() - 0.40).abs() < 1e-6);
    assert!((SearchOperator::Debug.target_temperature() - 0.20).abs() < 1e-6);
    assert!((SearchOperator::Crossover.target_temperature() - 0.50).abs() < 1e-6);

    let wrapped = "Here is the code:\n```python\ndef solve(x):\n    return x * 2\n```\nDone.";
    assert_eq!(
        PromptBuilder::extract_code_block(wrapped),
        "def solve(x):\n    return x * 2"
    );

    let thought_wrapped = "<|channel>thought\nLet's try ```python\nbad()\n``` first\n<channel|>```python\ndef solve(x):\n    return x + 1\n```";
    assert_eq!(
        PromptBuilder::extract_code_block(thought_wrapped),
        "def solve(x):\n    return x + 1"
    );
}

#[test]
fn sandbox_metric_parser_extracts_last_marker() {
    let logs = "Loading data...\nRSI_METRIC_JSON={\"status\":\"SUCCESS\",\"score\":91.25}\n";
    let (status, score) = SandboxRunner::parse_evaluator_output(logs);
    assert_eq!(status, "SUCCESS");
    assert!((score - 91.25).abs() < 1e-6);

    let bad_logs = "Segfault before metric output";
    let (bad_status, bad_score) = SandboxRunner::parse_evaluator_output(bad_logs);
    assert_eq!(bad_status, "METRIC_PARSE_ERROR");
    assert!((bad_score - 0.0).abs() < 1e-6);
}

#[test]
fn demo_seed_campaign_and_lineage_dag() {
    let mut ctx = test_ctx();
    let seed = dispatch(&mut ctx, Command::DemoSeed).unwrap();
    assert_eq!(seed["seeded"], true);
    assert_eq!(seed["cards_created"], 4);

    // Repeating demo_seed on non-empty DB is refused
    assert!(dispatch(&mut ctx, Command::DemoSeed).is_err());

    let frontier = dispatch(
        &mut ctx,
        Command::GetFrontier {
            task_id: "opt-synth".into(),
            limit: 10,
        },
    )
    .unwrap();
    let arr = frontier["frontier"].as_array().unwrap();
    assert_eq!(arr.len(), 4);
    assert!(arr[0]["primary_score"].as_f64().unwrap() >= arr[1]["primary_score"].as_f64().unwrap());

    let lineage = dispatch(
        &mut ctx,
        Command::Lineage {
            task_id: "opt-synth".into(),
        },
    )
    .unwrap();
    assert_eq!(lineage["nodes_count"], 4);
    assert!(lineage["edges_count"].as_u64().unwrap() >= 3);
}

#[test]
fn direct_evaluate_detects_compile_and_runtime_errors() {
    let mut ctx = test_ctx();
    dispatch(
        &mut ctx,
        Command::TaskUpsert {
            task_id: "t1".into(),
            name: "Test".into(),
            description: "Write solve(x)".into(),
            evaluator_script: "print('RSI_METRIC_JSON={\"status\":\"SUCCESS\",\"score\":1.0}')"
                .into(),
        },
    )
    .unwrap();

    let bad = Command::from_tool(
        "rsi_evaluate",
        &json!({
            "task_id": "t1",
            "code": "not valid function",
            "record": false
        }),
    )
    .unwrap();
    let bad_res = dispatch(&mut ctx, bad).unwrap();
    assert_eq!(bad_res["status"], "COMPILE_ERROR");

    let crash = Command::from_tool(
        "rsi_evaluate",
        &json!({
            "task_id": "t1",
            "code": "def solve(x):\n    return 1 / 0",
            "record": true
        }),
    )
    .unwrap();
    let crash_res = dispatch(&mut ctx, crash).unwrap();
    assert_eq!(crash_res["execution_status"], "RUNTIME_ERROR");
}

#[test]
fn multitab_help_supports_all_tabs_and_substring_search() {
    let mut ctx = test_ctx();
    let all = dispatch(
        &mut ctx,
        Command::Help {
            tab: None,
            search: None,
        },
    )
    .unwrap();
    assert_eq!(all["matched_tabs_count"], 9);

    let tsinghua = dispatch(
        &mut ctx,
        Command::Help {
            tab: Some("tsinghua_openrsi".into()),
            search: None,
        },
    )
    .unwrap();
    assert_eq!(tsinghua["matched_tabs_count"], 1);
    assert_eq!(tsinghua["tabs"][0]["id"], "tsinghua_openrsi");

    let filtered = dispatch(
        &mut ctx,
        Command::Help {
            tab: None,
            search: Some("Kadane".into()),
        },
    )
    .unwrap();
    assert!(filtered["matched_tabs_count"].as_u64().unwrap() >= 1);

    let bad = dispatch(
        &mut ctx,
        Command::Help {
            tab: Some("nonexistent".into()),
            search: None,
        },
    );
    assert!(bad.is_err());
}

#[test]
fn unified_diff_and_command_diff() {
    let mut ctx = test_ctx();
    dispatch(&mut ctx, Command::DemoSeed).unwrap();

    let cards_val = dispatch(
        &mut ctx,
        Command::CardList {
            task_id: "opt-synth".into(),
            operator: None,
            status: None,
            limit: 10,
        },
    )
    .unwrap();
    let cards = cards_val["cards"].as_array().unwrap();
    assert!(cards.len() >= 2);
    let c1 = cards[0]["card_id"].as_str().unwrap();
    let c2 = cards[1]["card_id"].as_str().unwrap();

    let diff = dispatch(
        &mut ctx,
        Command::Diff {
            card_a_id: c1.into(),
            card_b_id: c2.into(),
        },
    )
    .unwrap();
    assert_eq!(diff["card_a_id"], c1);
    assert_eq!(diff["card_b_id"], c2);
    let diff_text = diff["diff_text"].as_str().unwrap();
    assert!(diff_text.contains("---"));
    assert!(diff_text.contains("+++"));
}

#[test]
fn hyperparams_get_and_set() {
    let mut ctx = test_ctx();
    let hp = dispatch(&mut ctx, Command::HyperparamsGet).unwrap();
    assert!((hp["lambda_s"].as_f64().unwrap() - 0.50).abs() < 1e-6);

    let updated = dispatch(
        &mut ctx,
        Command::HyperparamsSet {
            w_score: Some(0.60),
            w_delta: Some(0.30),
            w_novelty: Some(0.10),
        },
    )
    .unwrap();
    assert!((updated["lambda_s"].as_f64().unwrap() - 0.60).abs() < 1e-6);
    assert!((updated["lambda_delta"].as_f64().unwrap() - 0.30).abs() < 1e-6);
    assert!((updated["lambda_nu"].as_f64().unwrap() - 0.10).abs() < 1e-6);

    // Negative weights rejected
    let bad = dispatch(
        &mut ctx,
        Command::HyperparamsSet {
            w_score: Some(-0.1),
            w_delta: None,
            w_novelty: None,
        },
    );
    assert!(bad.is_err());
}

#[test]
fn task_export_and_import_and_delete_prune() {
    let mut ctx = test_ctx();
    dispatch(&mut ctx, Command::DemoSeed).unwrap();

    // Export bundle
    let bundle = dispatch(
        &mut ctx,
        Command::Export {
            task_id: "opt-synth".into(),
        },
    )
    .unwrap();
    assert_eq!(bundle["version"], 1);
    assert_eq!(bundle["task"]["task_id"], "opt-synth");
    assert_eq!(bundle["cards"].as_array().unwrap().len(), 4);

    // Prune cards keeping 2
    let prune_res = dispatch(
        &mut ctx,
        Command::CardsPrune {
            task_id: "opt-synth".into(),
            keep_frontier: 2,
        },
    )
    .unwrap();
    assert_eq!(prune_res["cards_remaining"], 2);

    // Delete task
    let del = dispatch(
        &mut ctx,
        Command::TaskDelete {
            task_id: "opt-synth".into(),
        },
    )
    .unwrap();
    assert_eq!(del["deleted"], true);

    let list = dispatch(&mut ctx, Command::TaskList).unwrap();
    assert_eq!(list["count"], 0);

    // Import bundle back
    let imp = dispatch(&mut ctx, Command::Import { bundle }).unwrap();
    assert_eq!(imp["imported_cards"], 4);

    let list2 = dispatch(&mut ctx, Command::TaskList).unwrap();
    assert_eq!(list2["count"], 1);
}

#[test]
fn multi_language_rust_candidate_detection_and_evaluation() {
    assert!(SandboxRunner::is_rust_candidate(
        "pub fn solve(x: i32) -> i32 { x * 2 }",
        "extern crate candidate; fn main() {}"
    ));
    assert!(!SandboxRunner::is_rust_candidate(
        "def solve(x):\n    return x * 2",
        "import json, candidate"
    ));

    // Demo evaluation on Rust candidate
    let res = SandboxRunner::run_demo_evaluation(
        "pub fn solve(x: i32) -> i32 { x * 2 }",
        SearchOperator::Draft,
        None,
        1,
    );
    assert_eq!(res.status, "SUCCESS");
    assert!(res.primary_score > 70.0);

    // Demo panic detection
    let panic_res = SandboxRunner::run_demo_evaluation(
        "pub fn solve() { panic!(\"boom\"); }",
        SearchOperator::Draft,
        None,
        1,
    );
    assert_eq!(panic_res.status, "RUNTIME_ERROR");
}

#[test]
fn code_extraction_handles_rust_untagged_and_truncated_fences() {
    // The fence info string must never leak into the candidate source.
    let rust = "Sure:\n```rust\npub fn solve(x: i32) -> i32 { x * 2 }\n```";
    assert_eq!(
        PromptBuilder::extract_code_block(rust),
        "pub fn solve(x: i32) -> i32 { x * 2 }"
    );

    let untagged = "```\ndef solve(x):\n    return x\n```";
    assert_eq!(
        PromptBuilder::extract_code_block(untagged),
        "def solve(x):\n    return x"
    );

    // A tagged code block wins over an earlier non-code block.
    let mixed = "```text\nplan: double it\n```\n```python\ndef solve(x):\n    return x * 2\n```";
    assert_eq!(
        PromptBuilder::extract_code_block(mixed),
        "def solve(x):\n    return x * 2"
    );

    // Reply cut off at the token limit: no closing fence.
    let truncated = "```python\ndef solve(x):\n    return x";
    assert_eq!(
        PromptBuilder::extract_code_block(truncated),
        "def solve(x):\n    return x"
    );

    // A fence nested in a markdown list is indented as a whole.
    let nested = "1. Draft:\n    ```python\n    import math\n\n    def solve(x):\n        return x\n    ```\n";
    assert_eq!(
        PromptBuilder::extract_code_block(nested),
        "import math\n\ndef solve(x):\n    return x"
    );

    let bare = "def solve(x):\n    return x";
    assert_eq!(PromptBuilder::extract_code_block(bare), bare);
}

#[test]
fn prompts_follow_the_task_language() {
    let rust_eval = "extern crate candidate;\nfn main() { println!(\"{}\", candidate::solve(2)); }";
    assert_eq!(
        CodeLanguage::from_evaluator(rust_eval),
        Some(CodeLanguage::Rust)
    );
    assert_eq!(
        CodeLanguage::from_evaluator("import json, candidate"),
        Some(CodeLanguage::Python)
    );
    assert_eq!(CodeLanguage::from_evaluator("  "), None);

    let sys = PromptBuilder::build_system_prompt(SearchOperator::Draft, CodeLanguage::Rust);
    assert!(sys.contains("```rust"));
    assert!(sys.contains("`pub`"));
    assert!(!sys.contains("Python"));
    assert!(!sys.contains("%LANG%"));

    let user = PromptBuilder::build_user_prompt(
        SearchOperator::Improve,
        CodeLanguage::Python,
        "spec",
        Some("def solve(): pass"),
        None,
        None,
    );
    assert!(user.contains("```python\ndef solve(): pass"));

    // A Python candidate that merely mentions `fn ` is still Python when the
    // evaluator is Python.
    assert!(!SandboxRunner::is_rust_candidate(
        "x = 'pub fn solve'",
        "import json, candidate"
    ));
}

#[test]
fn metric_parser_normalizes_unknown_status_and_log_tail_is_char_safe() {
    let (status, score) =
        SandboxRunner::parse_evaluator_output("RSI_METRIC_JSON={\"status\":\"OK\",\"score\":9.0}");
    assert_eq!(status, "METRIC_PARSE_ERROR");
    assert!(score.abs() < 1e-9);

    let (status, score) = SandboxRunner::parse_evaluator_output(
        "RSI_METRIC_JSON={\"status\":\"RUNTIME_ERROR\",\"score\":0.0}",
    );
    assert_eq!(status, "RUNTIME_ERROR");
    assert!(score.abs() < 1e-9);

    assert_eq!(sandbox::tail_str("abcdef", 3), "def");
    assert_eq!(sandbox::tail_str("ab", 3), "ab");
    // Never split a multi-byte character.
    assert_eq!(sandbox::tail_str("a\u{00e9}b", 2), "b");
}

#[test]
fn operators_refuse_to_run_without_the_parents_they_need() {
    let mut ctx = test_ctx();
    dispatch(
        &mut ctx,
        Command::TaskUpsert {
            task_id: "t".into(),
            name: "T".into(),
            description: "Write solve(x)".into(),
            evaluator_script: "import candidate".into(),
        },
    )
    .unwrap();
    let step = |op| Command::Step {
        task_id: "t".into(),
        operator: op,
        provider: "demo".into(),
        sandbox_mode: "demo".into(),
        timeout_secs: 5,
    };

    assert!(dispatch(&mut ctx, step(SearchOperator::Improve)).is_err());
    assert!(dispatch(&mut ctx, step(SearchOperator::Crossover)).is_err());

    dispatch(&mut ctx, step(SearchOperator::Draft)).unwrap();
    // One card: CROSSOVER must not pair it with itself.
    assert!(dispatch(&mut ctx, step(SearchOperator::Crossover)).is_err());

    dispatch(&mut ctx, step(SearchOperator::Improve)).unwrap();
    let cross = dispatch(&mut ctx, step(SearchOperator::Crossover)).unwrap();
    assert_ne!(cross["parent_a_id"], cross["parent_b_id"]);
    assert!(cross["parent_a_id"].is_string() && cross["parent_b_id"].is_string());
}

#[test]
fn campaign_adapts_schedule_and_never_debugs_working_code() {
    let mut ctx = test_ctx();
    dispatch(
        &mut ctx,
        Command::TaskUpsert {
            task_id: "t".into(),
            name: "T".into(),
            description: "Write solve(x)".into(),
            evaluator_script: "import candidate".into(),
        },
    )
    .unwrap();
    let res = dispatch(
        &mut ctx,
        Command::Campaign {
            task_id: "t".into(),
            steps: 8,
            provider: "demo".into(),
            sandbox_mode: "demo".into(),
            timeout_secs: 5,
        },
    )
    .unwrap();
    assert_eq!(res["steps_executed"], 8);
    assert_eq!(res["aborted"], false);
    let steps = res["steps"].as_array().unwrap();
    assert_eq!(steps[0]["operator"], "DRAFT");
    // The demo oracle never fails, so there is never a faulty card to DEBUG.
    assert!(steps.iter().all(|s| s["operator"] != "DEBUG"));
    for s in steps.iter().filter(|s| s["operator"] == "CROSSOVER") {
        assert_ne!(s["parent_a_id"], s["parent_b_id"]);
    }
}

#[test]
fn hyperparams_survive_a_restart() {
    let dir = std::env::temp_dir().join(format!("rsicraft-test-{}", uuid::Uuid::new_v4()));
    let db = dir.join("hp.db");
    {
        let mut ctx = Ctx::new(Registry::open(&db).unwrap(), None, None, None);
        dispatch(
            &mut ctx,
            Command::HyperparamsSet {
                w_score: Some(0.2),
                w_delta: Some(0.3),
                w_novelty: Some(0.5),
            },
        )
        .unwrap();
    }
    {
        let mut ctx = Ctx::new(Registry::open(&db).unwrap(), None, None, None);
        let hp = dispatch(&mut ctx, Command::HyperparamsGet).unwrap();
        assert!((hp["lambda_s"].as_f64().unwrap() - 0.2).abs() < 1e-9);
        assert!((hp["lambda_nu"].as_f64().unwrap() - 0.5).abs() < 1e-9);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn diff_trims_common_context_and_stays_correct() {
    let a = "def solve(arr):\n    best = 0\n    for x in arr:\n        best = max(best, x)\n    return best";
    let b = "def solve(arr):\n    best = 0\n    for x in arr:\n        if x > best:\n            best = x\n    return best";
    let (text, added, removed) = generate_unified_diff("a.py", a, "b.py", b);
    assert_eq!((added, removed), (2, 1));
    assert!(text.contains(" def solve(arr):\n"));
    assert!(text.contains("-        best = max(best, x)\n"));
    assert!(text.contains("+        if x > best:\n+            best = x\n"));
    assert!(text.ends_with("     return best\n"));

    // Pure append and pure prefix removal.
    let (_, added, removed) = generate_unified_diff("a", "x\ny", "b", "x\ny\nz");
    assert_eq!((added, removed), (1, 0));
    let (_, added, removed) = generate_unified_diff("a", "w\nx\ny", "b", "x\ny");
    assert_eq!((added, removed), (0, 1));
}

#[test]
fn dataset_bandit_and_honest_training_gate() {
    let mut ctx = test_ctx();
    dispatch(&mut ctx, Command::DemoSeed).unwrap();

    // Rung 2: Self-Evolution Trace Curation
    let dataset = dispatch(
        &mut ctx,
        Command::DatasetExport {
            task_id: "opt-synth".into(),
            format: "all".into(),
            min_delta: 0.0,
        },
    )
    .unwrap();
    let sft = dataset["sft_traces"].as_array().unwrap();
    assert!(!sft.is_empty(), "expected SFT traces from demo seed cards");
    assert!(dataset["summary"]["sft_traces_count"].as_u64().unwrap() >= 1);

    // Rung 3: Meta-Evolution Multi-Armed Bandit
    let bandit = dispatch(
        &mut ctx,
        Command::BanditStatus {
            task_id: "opt-synth".into(),
        },
    )
    .unwrap();
    let arms = bandit["arms"].as_array().unwrap();
    assert_eq!(
        arms.len(),
        4,
        "expected 4 operator arms (DRAFT, IMPROVE, DEBUG, CROSSOVER)"
    );
    assert!(bandit["recommended_operator"].as_str().is_some());

    // Training: a dry run reports the dataset and changes nothing.
    let train = dispatch(
        &mut ctx,
        Command::TrainStep {
            task_id: "opt-synth".into(),
            epochs: 3,
            learning_rate: 0.0002,
            dry_run: true,
        },
    )
    .unwrap();
    assert_eq!(train["trained"], false);
    assert!(train["sft_samples"].as_u64().unwrap() >= 1);
    assert!(train["adapter_path"].is_null());
    let task = ctx.reg.get_task("opt-synth").unwrap().unwrap();
    assert_eq!(
        task.active_model_gen, 0,
        "a dry run must not advance the generation"
    );

    // Promotion gate: refuses to "compare" models on the demo oracle, and
    // refuses to run without an explicit candidate. No fake promotions.
    let gate = |sandbox: &str, candidate: Option<&str>| Command::PromoteGate {
        task_id: "opt-synth".into(),
        trials: 3,
        steps: 2,
        sandbox_mode: sandbox.into(),
        timeout_secs: 5,
        baseline_alias: None,
        candidate_alias: candidate.map(str::to_string),
    };
    assert!(dispatch(&mut ctx, gate("demo", Some("frontis-g1"))).is_err());
    assert!(dispatch(&mut ctx, gate("local", None)).is_err());
    let task = ctx.reg.get_task("opt-synth").unwrap().unwrap();
    assert_eq!(task.active_model_gen, 0);
}

#[test]
fn welch_statistic_separates_clear_wins_from_noise() {
    assert!(welch_t(&[1.0, 1.1, 0.9], &[2.0, 2.1, 1.9]) > GATE_T_THRESHOLD);
    assert!(welch_t(&[1.0, 1.4, 0.6], &[1.1, 0.7, 1.3]).abs() < GATE_T_THRESHOLD);
    assert!(welch_t(&[2.0, 2.1, 1.9], &[1.0, 1.1, 0.9]) < 0.0);
    assert!(welch_t(&[1.0, 1.0, 1.0], &[1.0, 1.0, 1.0]).abs() < 1e-12);
}

#[test]
fn step_phases_compose_to_the_same_result_as_dispatch() {
    let mut ctx = test_ctx();
    dispatch(&mut ctx, Command::DemoSeed).unwrap();
    // plan (store read) -> run (no store, movable to a worker thread) -> commit.
    let plan = plan_step(
        &ctx,
        "opt-synth",
        SearchOperator::Improve,
        "demo",
        "demo",
        5,
    )
    .unwrap();
    assert!(plan.parent_a_id.is_some());
    let (llm, sandbox) = (
        ctx.llm.clone(),
        SandboxRunner::new(ctx.sandbox.image_tag.clone()),
    );
    let outcome = std::thread::spawn(move || run_step(&plan, &llm, &sandbox).map(|o| (plan, o)))
        .join()
        .unwrap()
        .unwrap();
    let card = commit_step(&ctx, &outcome.0, &outcome.1).unwrap();
    assert_eq!(card["operator"], "IMPROVE");
    assert_eq!(ctx.reg.status().unwrap().cards_count, 5);
    assert!(plan_step(&ctx, "opt-synth", SearchOperator::Draft, "bogus", "demo", 5).is_err());
    assert!(plan_step(&ctx, "opt-synth", SearchOperator::Draft, "demo", "bogus", 5).is_err());
}

#[test]
fn promote_gate_trials_clamp_matches_the_schema() {
    let low = Command::from_tool("rsi_promote_gate", &json!({"trials": 1, "steps": 0}));
    let high = Command::from_tool("rsi_promote_gate", &json!({"trials": 100, "steps": 1000}));
    assert!(matches!(
        low,
        Ok(Command::PromoteGate {
            trials: 3,
            steps: 1,
            timeout_secs: 30,
            ref sandbox_mode,
            ..
        }) if sandbox_mode == "local"
    ));
    assert!(matches!(
        high,
        Ok(Command::PromoteGate {
            trials: 20,
            steps: 64,
            ..
        })
    ));
}

#[test]
fn help_api_tab_does_not_claim_a_finished_ladder() {
    let help = help::query_help(Some("api"), None).ok();
    let text = help.map(|v| v.to_string()).unwrap_or_default();
    assert!(text.contains("campaigns do not use it"), "{text}");
    assert!(text.contains("does not change the active model"), "{text}");
    assert!(!text.contains("Thompson"), "{text}");
    assert!(!text.contains("advancing active model"), "{text}");
}
